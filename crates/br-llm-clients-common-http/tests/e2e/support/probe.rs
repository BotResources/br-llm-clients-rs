use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use br_llm_clients_common_http::Endpoint;
use bytes::Bytes;
use h2::Reason;
use tokio::net::TcpListener;
use tokio::sync::watch;
use tokio::task::JoinSet;

use super::pki::Pki;
use super::probe_serve::serve;

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

pub(super) type Accepted = (
    http::Request<h2::RecvStream>,
    h2::server::SendResponse<Bytes>,
);

pub(super) type Queued = VecDeque<Accepted>;

#[derive(Clone)]
pub(super) struct Records {
    arrivals: Arc<Mutex<Vec<String>>>,
    arrived: Arc<Mutex<Vec<Arrived>>>,
    processed: Arc<Mutex<Vec<String>>>,
    refusals: Arc<AtomicUsize>,
    noted: Arc<watch::Sender<usize>>,
}

impl Records {
    fn new() -> Self {
        Self {
            arrivals: Arc::new(Mutex::new(Vec::new())),
            arrived: Arc::new(Mutex::new(Vec::new())),
            processed: Arc::new(Mutex::new(Vec::new())),
            refusals: Arc::new(AtomicUsize::new(0)),
            noted: Arc::new(watch::channel(0).0),
        }
    }

    pub(super) fn note_arrival(&self, id: String) {
        self.arrivals.lock().expect("lock").push(id);
        self.noted.send_modify(|noted| *noted += 1);
    }

    pub(super) fn note_arrived(&self, arrived: Arrived) {
        self.arrived.lock().expect("lock").push(arrived);
    }

    pub(super) fn note_processed(&self, id: String) {
        self.processed.lock().expect("lock").push(id);
    }

    pub(super) fn is_first_copy(&self, id: &str) -> bool {
        let arrivals = self.arrivals.lock().expect("lock");
        arrivals.iter().filter(|seen| *seen == id).count() == 1
    }

    pub(super) fn count_refusal(&self) -> usize {
        self.refusals.fetch_add(1, Ordering::SeqCst)
    }
}

pub struct Probe {
    pub addr: SocketAddr,
    pub accepted: Arc<AtomicUsize>,
    records: Records,
    task: tokio::task::JoinHandle<()>,
}

impl Probe {
    pub async fn start(script: Script) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let accepted = Arc::new(AtomicUsize::new(0));
        let records = Records::new();
        let acceptor = Pki::trusted().acceptor(&[b"h2"]);
        let (count, shared) = (Arc::clone(&accepted), records.clone());
        let task = tokio::spawn(async move {
            let mut connections = JoinSet::new();
            loop {
                let Ok((tcp, _)) = listener.accept().await else {
                    break;
                };
                let index = count.fetch_add(1, Ordering::SeqCst);
                let (acceptor, records) = (acceptor.clone(), shared.clone());
                connections.spawn(async move {
                    if let Ok(tls) = acceptor.accept(tcp).await {
                        serve(script, index, tls, records).await;
                    }
                });
            }
        });
        Self {
            addr,
            accepted,
            records,
            task,
        }
    }

    pub fn endpoint(&self) -> Endpoint {
        Endpoint::https(&format!("https://127.0.0.1:{}/x", self.addr.port())).expect("endpoint")
    }

    pub fn arrived(&self) -> Vec<Arrived> {
        self.records.arrived.lock().expect("lock").clone()
    }

    pub fn arrivals(&self) -> Vec<String> {
        self.records.arrivals.lock().expect("lock").clone()
    }

    pub async fn wait_arrivals(&self, count: usize) {
        let _ = self
            .records
            .noted
            .subscribe()
            .wait_for(|noted| *noted >= count)
            .await;
    }

    pub fn processed(&self) -> Vec<String> {
        self.records.processed.lock().expect("lock").clone()
    }
}

impl Drop for Probe {
    fn drop(&mut self) {
        self.task.abort();
    }
}
