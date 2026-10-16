use std::convert::Infallible;
use std::net::SocketAddr;
use std::pin::pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use br_llm_clients_common_http::Endpoint;
use hyper::body::Incoming;
use hyper::server::conn::{http1, http2};
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::net::TcpListener;
use tokio::sync::Notify;
use tokio::task::JoinSet;

use super::pki::Pki;
use super::script_body::{ScriptBody, Step};

#[derive(Clone, Copy)]
pub struct Flip {
    pub http1_connections: usize,
    pub http1_responds_after: Duration,
    pub http1_closes_after_responding: Duration,
    pub http2_accepts_after: Duration,
    pub http2_settings_after: Option<Duration>,
}

pub struct FlipServer {
    pub addr: SocketAddr,
    accepted: Arc<AtomicUsize>,
    task: tokio::task::JoinHandle<()>,
}

impl FlipServer {
    pub async fn start(flip: Flip) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let accepted = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&accepted);
        let http1_acceptor = Pki::trusted().acceptor(&[b"http/1.1"]);
        let http2_acceptor = Pki::trusted().acceptor(&[b"h2"]);
        let task = tokio::spawn(async move {
            let mut connections = JoinSet::new();
            while let Ok((tcp, _)) = listener.accept().await {
                let index = counter.fetch_add(1, Ordering::SeqCst);
                if index < flip.http1_connections {
                    let acceptor = http1_acceptor.clone();
                    connections.spawn(async move {
                        if let Ok(tls) = acceptor.accept(tcp).await {
                            serve_http1(TokioIo::new(tls), flip).await;
                        }
                    });
                } else {
                    let acceptor = http2_acceptor.clone();
                    connections.spawn(async move {
                        tokio::time::sleep(flip.http2_accepts_after).await;
                        let Ok(tls) = acceptor.accept(tcp).await else {
                            return;
                        };
                        match flip.http2_settings_after {
                            Some(after) => tokio::time::sleep(after).await,
                            None => std::future::pending().await,
                        }
                        serve_http2(TokioIo::new(tls)).await;
                    });
                }
            }
        });
        Self {
            addr,
            accepted,
            task,
        }
    }

    pub fn accepted(&self) -> usize {
        self.accepted.load(Ordering::SeqCst)
    }

    pub fn endpoint(&self) -> Endpoint {
        Endpoint::https(&format!("https://127.0.0.1:{}/x", self.addr.port())).expect("endpoint")
    }
}

impl Drop for FlipServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn ok() -> Response<ScriptBody> {
    Response::new(ScriptBody::new([Step::Data(b"ok")]))
}

async fn serve_http1<IO>(io: IO, flip: Flip)
where
    IO: hyper::rt::Read + hyper::rt::Write + Unpin + Send + 'static,
{
    let responded = Arc::new(Notify::new());
    let signal = Arc::clone(&responded);
    let service = service_fn(move |_: Request<Incoming>| {
        let signal = Arc::clone(&signal);
        async move {
            tokio::time::sleep(flip.http1_responds_after).await;
            signal.notify_one();
            Ok::<_, Infallible>(ok())
        }
    });
    let mut connection = pin!(http1::Builder::new().serve_connection(io, service));
    tokio::select! {
        _ = connection.as_mut() => {}
        () = responded.notified() => {
            tokio::time::sleep(flip.http1_closes_after_responding).await;
            connection.as_mut().graceful_shutdown();
            let _ = connection.await;
        }
    }
}

async fn serve_http2<IO>(io: IO)
where
    IO: hyper::rt::Read + hyper::rt::Write + Unpin + Send + 'static,
{
    let service = service_fn(|_: Request<Incoming>| async { Ok::<_, Infallible>(ok()) });
    let mut builder = http2::Builder::new(TokioExecutor::new());
    builder.max_concurrent_streams(Some(100));
    let _ = builder.serve_connection(io, service).await;
}
