use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use bytes::Bytes;
use h2::Reason;
use tokio::sync::watch;

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
    pub(super) fn new() -> Self {
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

    pub(super) fn arrived(&self) -> Vec<Arrived> {
        self.arrived.lock().expect("lock").clone()
    }

    pub(super) fn arrivals(&self) -> Vec<String> {
        self.arrivals.lock().expect("lock").clone()
    }

    pub(super) fn processed(&self) -> Vec<String> {
        self.processed.lock().expect("lock").clone()
    }

    pub(super) async fn wait_arrivals(&self, count: usize) {
        let _ = self
            .noted
            .subscribe()
            .wait_for(|noted| *noted >= count)
            .await;
    }

    pub(super) fn count_refusal(&self) -> usize {
        self.refusals.fetch_add(1, Ordering::SeqCst)
    }
}
