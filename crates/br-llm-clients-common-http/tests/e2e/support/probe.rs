use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use br_llm_clients_common_http::Endpoint;
use bytes::Bytes;
use h2::Reason;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinSet;

use super::frames::{ACK, HEADERS, SETTINGS, frame, goaway_frame, read_frame, settings_frame};
use super::pki::Pki;

#[derive(Clone, Copy)]
pub enum Script {
    RefuseFirst(usize),
    RefuseAll,
    GoAway {
        first_connections: usize,
        last_stream: u32,
        code: u32,
    },
    CloseAfterReceiving,
    ResetAll(Reason),
    GoAwayAfterReceiving,
    ZeroWindowClose,
    ZeroWindowGoAway,
    RefuseEvery(usize),
    Graceful(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arrived {
    pub id: String,
    pub key: Option<String>,
    pub body: Option<Vec<u8>>,
}

type Accepted = (
    http::Request<h2::RecvStream>,
    h2::server::SendResponse<Bytes>,
);

pub struct Probe {
    pub addr: SocketAddr,
    pub accepted: Arc<AtomicUsize>,
    pub arrivals: Arc<Mutex<Vec<String>>>,
    pub arrived: Arc<Mutex<Vec<Arrived>>>,
    pub processed: Arc<Mutex<Vec<String>>>,
    task: tokio::task::JoinHandle<()>,
}

impl Probe {
    pub async fn start(script: Script) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let accepted = Arc::new(AtomicUsize::new(0));
        let arrivals = Arc::new(Mutex::new(Vec::new()));
        let arrived = Arc::new(Mutex::new(Vec::new()));
        let processed = Arc::new(Mutex::new(Vec::new()));
        let refusals = Arc::new(AtomicUsize::new(0));
        let acceptor = Pki::trusted().acceptor(&[b"h2"]);
        let (count, seen, done, full) = (
            Arc::clone(&accepted),
            Arc::clone(&arrivals),
            Arc::clone(&processed),
            Arc::clone(&arrived),
        );
        let task = tokio::spawn(async move {
            let mut connections = JoinSet::new();
            loop {
                let Ok((tcp, _)) = listener.accept().await else {
                    break;
                };
                let index = count.fetch_add(1, Ordering::SeqCst);
                let (acceptor, seen, done, refusals, full) = (
                    acceptor.clone(),
                    Arc::clone(&seen),
                    Arc::clone(&done),
                    Arc::clone(&refusals),
                    Arc::clone(&full),
                );
                connections.spawn(async move {
                    let Ok(tls) = acceptor.accept(tcp).await else {
                        return;
                    };
                    if let Script::GoAway {
                        first_connections,
                        last_stream,
                        code,
                    } = script
                        && index < first_connections
                    {
                        if go_away_with(tls, last_stream, code, b"").await {
                            seen.lock().expect("lock").push("?".to_owned());
                        }
                        return;
                    }
                    let mut builder = h2::server::Builder::new();
                    builder.max_concurrent_streams(1000);
                    if matches!(script, Script::ZeroWindowClose | Script::ZeroWindowGoAway) {
                        builder.initial_window_size(0);
                    }
                    let Ok(mut conn) = builder.handshake::<_, Bytes>(tls).await else {
                        return;
                    };
                    let mut served = 0usize;
                    let mut queued: VecDeque<Accepted> = VecDeque::new();
                    loop {
                        let next = match queued.pop_front() {
                            Some(next) => next,
                            None => match conn.accept().await {
                                Some(Ok(next)) => next,
                                _ => break,
                            },
                        };
                        let (request, mut respond) = next;
                        served = served.saturating_add(1);
                        if matches!(script, Script::Graceful(after) if after == served) {
                            conn.graceful_shutdown();
                        }
                        let id = request
                            .headers()
                            .get("x-id")
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or_default()
                            .to_owned();
                        seen.lock().expect("lock").push(id.clone());
                        let (parts, body) = request.into_parts();
                        let key = parts
                            .headers
                            .get("x-api-key")
                            .and_then(|v| v.to_str().ok())
                            .map(str::to_owned);
                        let body = if matches!(script, Script::RefuseFirst(_)) {
                            Some(read_body_driving(&mut conn, &mut queued, body).await)
                        } else {
                            None
                        };
                        full.lock().expect("lock").push(Arrived {
                            id: id.clone(),
                            key,
                            body,
                        });
                        let refuse = match script {
                            Script::RefuseAll => true,
                            Script::RefuseFirst(n) => refusals.fetch_add(1, Ordering::SeqCst) < n,
                            Script::RefuseEvery(n) => {
                                let arrivals = seen.lock().expect("lock");
                                let first_copy =
                                    arrivals.iter().filter(|seen| **seen == id).count() == 1;
                                first_copy
                                    && id
                                        .trim_start_matches('r')
                                        .parse::<usize>()
                                        .is_ok_and(|number| number % n == 0)
                            }
                            _ => false,
                        };
                        if refuse {
                            respond.send_reset(Reason::REFUSED_STREAM);
                            continue;
                        }
                        if let Script::ResetAll(reason) = script {
                            respond.send_reset(reason);
                            continue;
                        }
                        if matches!(
                            script,
                            Script::CloseAfterReceiving | Script::ZeroWindowClose
                        ) {
                            gather_more(&mut conn, &seen).await;
                            return;
                        }
                        if matches!(
                            script,
                            Script::GoAwayAfterReceiving | Script::ZeroWindowGoAway
                        ) {
                            gather_more(&mut conn, &seen).await;
                            conn.abrupt_shutdown(Reason::NO_ERROR);
                            while conn.accept().await.is_some() {}
                            return;
                        }
                        done.lock().expect("lock").push(id);
                        let ok = http::Response::builder()
                            .status(200)
                            .body(())
                            .expect("response");
                        if let Ok(mut stream) = respond.send_response(ok, false) {
                            let _ = stream.send_data(Bytes::from_static(b"ok"), true);
                        }
                    }
                });
            }
        });
        Self {
            addr,
            accepted,
            arrivals,
            arrived,
            processed,
            task,
        }
    }

    pub fn endpoint(&self) -> Endpoint {
        Endpoint::https(&format!("https://127.0.0.1:{}/x", self.addr.port())).expect("endpoint")
    }

    pub fn arrived(&self) -> Vec<Arrived> {
        self.arrived.lock().expect("lock").clone()
    }

    pub fn arrivals(&self) -> Vec<String> {
        self.arrivals.lock().expect("lock").clone()
    }

    pub fn processed(&self) -> Vec<String> {
        self.processed.lock().expect("lock").clone()
    }
}

async fn read_body_driving<T>(
    conn: &mut h2::server::Connection<T, Bytes>,
    queued: &mut VecDeque<Accepted>,
    mut body: h2::RecvStream,
) -> Vec<u8>
where
    T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let mut flow = body.flow_control().clone();
    let mut bytes = Vec::new();
    let mut accepting = true;
    loop {
        tokio::select! {
            chunk = body.data() => match chunk {
                Some(Ok(chunk)) => {
                    let _ = flow.release_capacity(chunk.len());
                    bytes.extend_from_slice(&chunk);
                }
                _ => return bytes,
            },
            accepted = conn.accept(), if accepting => match accepted {
                Some(Ok(next)) => queued.push_back(next),
                _ => accepting = false,
            },
        }
    }
}

async fn gather_more<T>(conn: &mut h2::server::Connection<T, Bytes>, seen: &Mutex<Vec<String>>)
where
    T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let mut held = Vec::new();
    let _ = tokio::time::timeout(std::time::Duration::from_millis(20), async {
        while let Some(Ok((request, respond))) = conn.accept().await {
            let id = request
                .headers()
                .get("x-id")
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default()
                .to_owned();
            seen.lock().expect("lock").push(id);
            held.push((request, respond));
        }
    })
    .await;
}

impl Drop for Probe {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub async fn go_away_with<S: AsyncReadExt + AsyncWriteExt + Unpin>(
    mut stream: S,
    last_stream: u32,
    code: u32,
    debug: &[u8],
) -> bool {
    let mut preface = [0u8; 24];
    if stream.read_exact(&mut preface).await.is_err() {
        return false;
    }
    let _ = stream.write_all(&settings_frame(None)).await;
    while let Some(received) = read_frame(&mut stream).await {
        if received.kind == SETTINGS && received.flags & ACK == 0 {
            let _ = stream.write_all(&frame(SETTINGS, ACK, 0, &[])).await;
        }
        if received.kind == HEADERS {
            let _ = stream
                .write_all(&goaway_frame(last_stream, code, debug))
                .await;
            let _ = stream.flush().await;
            if last_stream > 0 {
                return true;
            }
            let _ = stream.shutdown().await;
            let mut sink = [0u8; 1024];
            let lingering =
                async { while matches!(stream.read(&mut sink).await, Ok(n) if n > 0) {} };
            let _ = tokio::time::timeout(std::time::Duration::from_millis(500), lingering).await;
            return true;
        }
    }
    false
}
