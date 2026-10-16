use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use br_llm_clients_common_http::Endpoint;
use tokio::io::{AsyncWriteExt, split};
use tokio::net::TcpListener;
use tokio::sync::{Notify, mpsc};
use tokio::task::JoinSet;

use super::frames::{
    ACK, DATA, END_STREAM, HEADERS, PING, SETTINGS, empty_response, frame, goaway_frame,
    read_frame, read_preface, settings_frame, settings_frame_padded,
};
use super::pki::Pki;
pub use super::raw_h2_policy::{Announce, Answer, Ending, Policy, TlsFault};

const RST_STREAM: u8 = 3;
const WINDOW_UPDATE: u8 = 8;
const REFUSED_STREAM: u32 = 7;

pub struct RawH2 {
    pub addr: SocketAddr,
    accepted: Arc<AtomicUsize>,
    streams: Arc<AtomicUsize>,
    closed: Arc<AtomicUsize>,
    peak: Arc<AtomicUsize>,
    stop: Arc<Notify>,
    task: tokio::task::JoinHandle<()>,
}

struct Counters {
    opened: Arc<AtomicUsize>,
    closed: Arc<AtomicUsize>,
    peak: Arc<AtomicUsize>,
}

impl RawH2 {
    pub async fn start(policy: impl Fn(usize) -> Policy + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let accepted = Arc::new(AtomicUsize::new(0));
        let streams = Arc::new(AtomicUsize::new(0));
        let closed = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(Notify::new());
        let stopped = Arc::clone(&stop);
        let acceptor = Pki::trusted().acceptor(&[b"h2"]);
        let (count, opened) = (Arc::clone(&accepted), Arc::clone(&streams));
        let (ended, tallest) = (Arc::clone(&closed), Arc::clone(&peak));
        let task = tokio::spawn(async move {
            let mut connections = JoinSet::new();
            loop {
                let accepted = tokio::select! {
                    accepted = listener.accept() => accepted,
                    () = stopped.notified() => break,
                };
                let Ok((tcp, _)) = accepted else { break };
                let index = count.fetch_add(1, Ordering::SeqCst);
                let acceptor = acceptor.clone();
                let counters = Counters {
                    opened: Arc::clone(&opened),
                    closed: Arc::clone(&ended),
                    peak: Arc::clone(&tallest),
                };
                let policy = policy(index);
                connections.spawn(async move {
                    tokio::time::sleep(policy.accept_delay).await;
                    match policy.tls_fault {
                        TlsFault::Close => return,
                        TlsFault::Hang => std::future::pending::<()>().await,
                        TlsFault::None => {}
                    }
                    if let Ok(tls) = acceptor.accept(tcp).await {
                        if matches!(policy.ends_after, Some((_, Ending::Reset))) {
                            reset_on_close(tls.get_ref().0);
                        }
                        serve(tls, policy, counters).await;
                    }
                });
            }
            drop(listener);
            while connections.join_next().await.is_some() {}
        });
        Self {
            addr,
            accepted,
            streams,
            closed,
            peak,
            stop,
            task,
        }
    }

    pub fn refuse_new_connections(&self) {
        self.stop.notify_one();
    }

    pub fn closed(&self) -> usize {
        self.closed.load(Ordering::SeqCst)
    }

    pub fn peak_streams_on_one_connection(&self) -> usize {
        self.peak.load(Ordering::SeqCst)
    }

    pub fn accepted(&self) -> usize {
        self.accepted.load(Ordering::SeqCst)
    }

    pub fn streams(&self) -> usize {
        self.streams.load(Ordering::SeqCst)
    }

    pub fn endpoint(&self) -> Endpoint {
        Endpoint::https(&format!("https://127.0.0.1:{}/x", self.addr.port())).expect("endpoint")
    }
}

impl Drop for RawH2 {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[allow(deprecated)] // SO_LINGER 0 is the only portable way to make the kernel send a reset
fn reset_on_close(tcp: &tokio::net::TcpStream) {
    let _ = tcp.set_linger(Some(Duration::ZERO));
}

async fn serve<S>(tls: S, policy: Policy, counters: Counters)
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + 'static,
{
    let Counters {
        opened,
        closed,
        peak,
    } = counters;
    match policy.ends_after {
        Some((after, _)) => {
            let _ = tokio::time::timeout(after, session(tls, policy, opened, peak)).await;
        }
        None => session(tls, policy, opened, peak).await,
    }
    closed.fetch_add(1, Ordering::SeqCst);
}

async fn session<S>(tls: S, policy: Policy, opened: Arc<AtomicUsize>, peak: Arc<AtomicUsize>)
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + 'static,
{
    let (mut reader, mut writer) = split(tls);
    let (out, mut queue) = mpsc::unbounded_channel::<Vec<u8>>();
    tokio::spawn(async move {
        while let Some(bytes) = queue.recv().await {
            if writer.write_all(&bytes).await.is_err() || writer.flush().await.is_err() {
                break;
            }
        }
    });
    match policy.announce {
        Announce::Nothing => {}
        Announce::Unlimited => drop(out.send(settings_frame(None))),
        Announce::Limit(limit) => drop(out.send(settings_frame(Some(limit)))),
        Announce::Padded { filler, limit } => {
            drop(out.send(settings_frame_padded(filler, limit)));
        }
        Announce::Raw(bytes) => drop(out.send(bytes.to_vec())),
        Announce::Spaced(first, delay, then) => {
            drop(out.send(first.to_vec()));
            let out = out.clone();
            tokio::spawn(async move {
                tokio::time::sleep(delay).await;
                drop(out.send(then.to_vec()));
            });
        }
        Announce::RawAfter(delay, bytes) => {
            let out = out.clone();
            tokio::spawn(async move {
                tokio::time::sleep(delay).await;
                drop(out.send(bytes.to_vec()));
            });
        }
    }
    if let Some((delay, code)) = policy.goaway_after {
        let out = out.clone();
        tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            drop(out.send(goaway_frame(0, code, b"")));
        });
    }
    if !read_preface(&mut reader).await {
        return;
    }
    let mut seen = 0usize;
    let live = Arc::new(AtomicUsize::new(0));
    while let Some(received) = read_frame(&mut reader).await {
        match received.kind {
            SETTINGS if received.flags & ACK == 0 => drop(out.send(frame(SETTINGS, ACK, 0, &[]))),
            PING if received.flags & ACK == 0 => {
                drop(out.send(frame(PING, ACK, 0, &received.payload)));
            }
            DATA if !received.payload.is_empty() => {
                let credit = u32::try_from(received.payload.len()).unwrap_or(u32::MAX);
                drop(out.send(frame(WINDOW_UPDATE, 0, 0, &credit.to_be_bytes())));
                drop(out.send(frame(
                    WINDOW_UPDATE,
                    0,
                    received.stream,
                    &credit.to_be_bytes(),
                )));
            }
            _ => {}
        }
        if received.kind == HEADERS {
            seen += 1;
            opened.fetch_add(1, Ordering::SeqCst);
            let now = live.fetch_add(1, Ordering::SeqCst).saturating_add(1);
            peak.fetch_max(now, Ordering::SeqCst);
            if let Some(code) = policy.goaway_on_headers {
                drop(out.send(goaway_frame(0, code, b"")));
            }
            if let Some((limit, after)) = policy.lower_to_after
                && seen == after
            {
                drop(out.send(settings_frame(Some(limit))));
            }
        }
        if matches!(received.kind, HEADERS | DATA) && received.flags & END_STREAM != 0 {
            answer(&out, policy.answer, received.stream, &live);
        }
    }
}

fn answer(
    out: &mpsc::UnboundedSender<Vec<u8>>,
    answer: Answer,
    stream: u32,
    live: &Arc<AtomicUsize>,
) {
    match answer {
        Answer::Never => {}
        Answer::Frames(build) => {
            live.fetch_sub(1, Ordering::SeqCst);
            drop(out.send(build(stream)));
        }
        Answer::Refuse => {
            live.fetch_sub(1, Ordering::SeqCst);
            refuse(out, stream);
        }
        Answer::RefuseAfter(delay) => {
            let (out, live) = (out.clone(), Arc::clone(live));
            tokio::spawn(async move {
                tokio::time::sleep(delay).await;
                live.fetch_sub(1, Ordering::SeqCst);
                refuse(&out, stream);
            });
        }
        Answer::After(delay) => {
            let (out, live) = (out.clone(), Arc::clone(live));
            tokio::spawn(async move {
                tokio::time::sleep(delay).await;
                live.fetch_sub(1, Ordering::SeqCst);
                drop(out.send(empty_response(stream)));
            });
        }
    }
}

fn refuse(out: &mpsc::UnboundedSender<Vec<u8>>, stream: u32) {
    let reset = frame(RST_STREAM, 0, stream, &REFUSED_STREAM.to_be_bytes());
    drop(out.send(reset));
}
