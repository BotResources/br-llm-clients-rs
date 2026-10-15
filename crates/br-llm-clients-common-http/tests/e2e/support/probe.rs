use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use br_llm_clients_common_http::Endpoint;
use bytes::Bytes;
use h2::Reason;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinSet;

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
}

pub struct Probe {
    pub addr: SocketAddr,
    pub accepted: Arc<AtomicUsize>,
    pub arrivals: Arc<Mutex<Vec<String>>>,
    pub processed: Arc<Mutex<Vec<String>>>,
    task: tokio::task::JoinHandle<()>,
}

impl Probe {
    pub async fn start(script: Script) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let accepted = Arc::new(AtomicUsize::new(0));
        let arrivals = Arc::new(Mutex::new(Vec::new()));
        let processed = Arc::new(Mutex::new(Vec::new()));
        let refusals = Arc::new(AtomicUsize::new(0));
        let acceptor = Pki::trusted().acceptor(&[b"h2"]);
        let (count, seen, done) = (
            Arc::clone(&accepted),
            Arc::clone(&arrivals),
            Arc::clone(&processed),
        );
        let task = tokio::spawn(async move {
            let mut connections = JoinSet::new();
            loop {
                let Ok((tcp, _)) = listener.accept().await else {
                    break;
                };
                let index = count.fetch_add(1, Ordering::SeqCst);
                let (acceptor, seen, done, refusals) = (
                    acceptor.clone(),
                    Arc::clone(&seen),
                    Arc::clone(&done),
                    Arc::clone(&refusals),
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
                    let Ok(mut conn) = h2::server::Builder::new()
                        .max_concurrent_streams(1000)
                        .handshake::<_, Bytes>(tls)
                        .await
                    else {
                        return;
                    };
                    while let Some(Ok((request, mut respond))) = conn.accept().await {
                        let id = request
                            .headers()
                            .get("x-id")
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or_default()
                            .to_owned();
                        seen.lock().expect("lock").push(id.clone());
                        let refuse = match script {
                            Script::RefuseAll => true,
                            Script::RefuseFirst(n) => refusals.fetch_add(1, Ordering::SeqCst) < n,
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
                        if matches!(script, Script::CloseAfterReceiving) {
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
            processed,
            task,
        }
    }

    pub fn endpoint(&self) -> Endpoint {
        Endpoint::https(&format!("https://127.0.0.1:{}/x", self.addr.port())).expect("endpoint")
    }

    pub fn arrivals(&self) -> Vec<String> {
        self.arrivals.lock().expect("lock").clone()
    }

    pub fn processed(&self) -> Vec<String> {
        self.processed.lock().expect("lock").clone()
    }
}

impl Drop for Probe {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn frame(kind: u8, flags: u8, stream: u32, payload: &[u8]) -> Vec<u8> {
    let length = u32::try_from(payload.len()).expect("length");
    let mut out = vec![
        (length >> 16) as u8,
        (length >> 8) as u8,
        length as u8,
        kind,
        flags,
    ];
    out.extend_from_slice(&stream.to_be_bytes());
    out.extend_from_slice(payload);
    out
}

pub async fn read_frame<S: AsyncReadExt + Unpin>(stream: &mut S) -> Option<(u8, u8, Vec<u8>)> {
    let mut head = [0u8; 9];
    stream.read_exact(&mut head).await.ok()?;
    let length = (usize::from(head[0]) << 16) | (usize::from(head[1]) << 8) | usize::from(head[2]);
    let mut payload = vec![0u8; length];
    stream.read_exact(&mut payload).await.ok()?;
    Some((head[3], head[4], payload))
}

pub fn goaway_frame(last_stream: u32, code: u32, debug: &[u8]) -> Vec<u8> {
    let mut payload = last_stream.to_be_bytes().to_vec();
    payload.extend_from_slice(&code.to_be_bytes());
    payload.extend_from_slice(debug);
    frame(7, 0, 0, &payload)
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
    let _ = stream.write_all(&frame(4, 0, 0, &[])).await;
    while let Some((kind, flags, _)) = read_frame(&mut stream).await {
        if kind == 4 && flags & 1 == 0 {
            let _ = stream.write_all(&frame(4, 1, 0, &[])).await;
        }
        if kind == 1 {
            let _ = stream
                .write_all(&goaway_frame(last_stream, code, debug))
                .await;
            let _ = stream.flush().await;
            if last_stream > 0 {
                return true;
            }
            let mut sink = [0u8; 1024];
            while matches!(stream.read(&mut sink).await, Ok(n) if n > 0) {}
            return true;
        }
    }
    false
}
