use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use br_llm_clients_common_http::Endpoint;
use tokio::io::{AsyncWriteExt, split};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::task::JoinSet;

use super::frames::{
    ACK, DATA, END_STREAM, HEADERS, PING, SETTINGS, empty_response, frame, read_frame,
    read_preface, settings_frame, settings_frame_padded,
};
use super::pki::Pki;

const RST_STREAM: u8 = 3;
const WINDOW_UPDATE: u8 = 8;
const REFUSED_STREAM: u32 = 7;

#[derive(Clone, Copy)]
pub enum Announce {
    Nothing,
    Unlimited,
    Limit(u32),
    Padded { filler: usize, limit: u32 },
    Raw(&'static [u8]),
    RawAfter(Duration, &'static [u8]),
}

#[derive(Clone, Copy)]
pub enum Answer {
    After(Duration),
    Refuse,
    RefuseAfter(Duration),
    Never,
}

#[derive(Clone, Copy)]
pub struct Policy {
    pub announce: Announce,
    pub answer: Answer,
    pub lower_to_after: Option<(u32, usize)>,
}

impl Policy {
    pub fn healthy() -> Self {
        Self {
            announce: Announce::Limit(100),
            answer: Answer::After(Duration::ZERO),
            lower_to_after: None,
        }
    }

    pub fn announcing(announce: Announce) -> Self {
        Self {
            announce,
            ..Self::healthy()
        }
    }
}

pub struct RawH2 {
    pub addr: SocketAddr,
    accepted: Arc<AtomicUsize>,
    streams: Arc<AtomicUsize>,
    closed: Arc<AtomicUsize>,
    peak: Arc<AtomicUsize>,
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
        let acceptor = Pki::trusted().acceptor(&[b"h2"]);
        let (count, opened) = (Arc::clone(&accepted), Arc::clone(&streams));
        let (ended, tallest) = (Arc::clone(&closed), Arc::clone(&peak));
        let task = tokio::spawn(async move {
            let mut connections = JoinSet::new();
            while let Ok((tcp, _)) = listener.accept().await {
                let index = count.fetch_add(1, Ordering::SeqCst);
                let acceptor = acceptor.clone();
                let counters = Counters {
                    opened: Arc::clone(&opened),
                    closed: Arc::clone(&ended),
                    peak: Arc::clone(&tallest),
                };
                let policy = policy(index);
                connections.spawn(async move {
                    if let Ok(tls) = acceptor.accept(tcp).await {
                        serve(tls, policy, counters).await;
                    }
                });
            }
        });
        Self {
            addr,
            accepted,
            streams,
            closed,
            peak,
            task,
        }
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

async fn serve<S>(tls: S, policy: Policy, counters: Counters)
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
        Announce::RawAfter(delay, bytes) => {
            let out = out.clone();
            tokio::spawn(async move {
                tokio::time::sleep(delay).await;
                drop(out.send(bytes.to_vec()));
            });
        }
    }
    if !read_preface(&mut reader).await {
        return;
    }
    let mut seen = 0usize;
    let live = Arc::new(AtomicUsize::new(0));
    let Counters {
        opened,
        closed,
        peak,
    } = counters;
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
    closed.fetch_add(1, Ordering::SeqCst);
}

fn answer(
    out: &mpsc::UnboundedSender<Vec<u8>>,
    answer: Answer,
    stream: u32,
    live: &Arc<AtomicUsize>,
) {
    match answer {
        Answer::Never => {}
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
