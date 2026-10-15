use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use br_llm_clients_common_http::Endpoint;
use hyper::body::Incoming;
use hyper::server::conn::{http1, http2};
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::net::TcpListener;
use tokio::sync::Notify;
use tokio::task::JoinSet;
use tokio_rustls::TlsAcceptor;

use super::script_body::ScriptBody;

pub type Reply = Pin<Box<dyn Future<Output = Result<Response<ScriptBody>, io::Error>> + Send>>;
pub type Service = Arc<dyn Fn(http::request::Parts) -> Reply + Send + Sync>;

pub struct Options {
    pub max_streams: Option<u32>,
    pub handshake_delay: std::time::Duration,
    pub stall_first: usize,
}

pub struct TlsServer {
    pub addr: SocketAddr,
    accepted: Arc<AtomicUsize>,
    closed: Arc<AtomicUsize>,
    kill: Arc<Notify>,
    task: tokio::task::JoinHandle<()>,
}

impl TlsServer {
    pub async fn start(acceptor: TlsAcceptor, service: Service) -> Self {
        Self::start_limited(acceptor, service, None, std::time::Duration::ZERO).await
    }

    pub async fn start_limited(
        acceptor: TlsAcceptor,
        service: Service,
        max_streams: Option<u32>,
        handshake_delay: std::time::Duration,
    ) -> Self {
        let options = Options {
            max_streams,
            handshake_delay,
            stall_first: 0,
        };
        Self::start_with(acceptor, service, options).await
    }

    pub async fn start_with(acceptor: TlsAcceptor, service: Service, options: Options) -> Self {
        let Options {
            max_streams,
            handshake_delay,
            stall_first,
        } = options;
        let kill = Arc::new(Notify::new());
        let killed = Arc::clone(&kill);
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let accepted = Arc::new(AtomicUsize::new(0));
        let closed = Arc::new(AtomicUsize::new(0));
        let closed_counter = Arc::clone(&closed);
        let counter = Arc::clone(&accepted);
        let task = tokio::spawn(async move {
            let mut connections = JoinSet::new();
            loop {
                let Ok((tcp, _)) = listener.accept().await else {
                    break;
                };
                let index = counter.fetch_add(1, Ordering::SeqCst);
                let killed = Arc::clone(&killed);
                let acceptor = acceptor.clone();
                let service = Arc::clone(&service);
                let closed_counter = Arc::clone(&closed_counter);
                connections.spawn(async move {
                    if index < stall_first {
                        std::future::pending::<()>().await;
                    }
                    tokio::time::sleep(handshake_delay).await;
                    let Ok(tls) = acceptor.accept(tcp).await else {
                        return;
                    };
                    let h2 = tls.get_ref().1.alpn_protocol() == Some(b"h2".as_slice());
                    let io = TokioIo::new(tls);
                    let svc = service_fn(move |req: Request<Incoming>| {
                        let service = Arc::clone(&service);
                        async move {
                            let (parts, body) = req.into_parts();
                            drain(body).await;
                            service(parts).await
                        }
                    });
                    let serving = async {
                        if h2 {
                            let mut builder = http2::Builder::new(TokioExecutor::new());
                            builder.max_concurrent_streams(max_streams);
                            let _ = builder.serve_connection(io, svc).await;
                        } else {
                            let _ = http1::Builder::new().serve_connection(io, svc).await;
                        }
                    };
                    tokio::select! {
                        () = serving => {}
                        () = killed.notified() => {}
                    }
                    closed_counter.fetch_add(1, Ordering::SeqCst);
                });
            }
        });
        Self {
            addr,
            accepted,
            closed,
            kill,
            task,
        }
    }

    pub fn accepted(&self) -> usize {
        self.accepted.load(Ordering::SeqCst)
    }

    pub fn kill_connections(&self) {
        self.kill.notify_waiters();
    }

    pub fn closed(&self) -> usize {
        self.closed.load(Ordering::SeqCst)
    }

    pub fn endpoint(&self, path: &str) -> Endpoint {
        Endpoint::https(&format!("https://127.0.0.1:{}{}", self.addr.port(), path))
            .expect("endpoint")
    }
}

impl Drop for TlsServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub fn service<F, Fut>(f: F) -> Service
where
    F: Fn(http::request::Parts) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Response<ScriptBody>, io::Error>> + Send + 'static,
{
    Arc::new(move |req| Box::pin(f(req)))
}

async fn drain(mut body: Incoming) {
    while let Some(Ok(_)) =
        std::future::poll_fn(|cx| hyper::body::Body::poll_frame(std::pin::Pin::new(&mut body), cx))
            .await
    {}
}
