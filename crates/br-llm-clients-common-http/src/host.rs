use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tokio::sync::watch;
use tokio::time::Instant;

use crate::claim::Landing;
use crate::dialer::DialFailure;
use crate::h2state::H2State;
use crate::transport::{H1Conn, H2Sender, Signal};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Proto {
    H1,
    H2,
}

pub(crate) struct Idle {
    pub(crate) conn: H1Conn,
    pub(crate) since: Instant,
}

pub(crate) struct H2Entry {
    pub(crate) sender: H2Sender,
    pub(crate) state: Arc<H2State>,
}

pub(crate) struct Recorded {
    pub(crate) seq: u64,
    pub(crate) failure: DialFailure,
}

pub(crate) struct Host {
    pub(crate) proto: Option<Proto>,
    pub(crate) dialing: bool,
    pub(crate) landing: Arc<Landing>,
    pub(crate) h2: Vec<H2Entry>,
    pub(crate) h1: Vec<Idle>,
    pub(crate) live_h1: Arc<AtomicUsize>,
    pub(crate) waiting: Arc<AtomicUsize>,
    pub(crate) signal: Signal,
    pub(crate) failure: Option<Recorded>,
    failures: u64,
}

impl Host {
    pub(crate) fn new() -> Self {
        Self {
            proto: None,
            dialing: false,
            landing: Landing::new(),
            h2: Vec::new(),
            h1: Vec::new(),
            live_h1: Arc::new(AtomicUsize::new(0)),
            waiting: Arc::new(AtomicUsize::new(0)),
            signal: Arc::new(watch::channel(0u64).0),
            failure: None,
            failures: 0,
        }
    }

    pub(crate) fn in_use(&self) -> bool {
        self.dialing
            || self.waiting.load(Ordering::SeqCst) > 0
            || !self.h2.is_empty()
            || !self.h1.is_empty()
            || self.live_h1.load(Ordering::SeqCst) > 0
    }

    pub(crate) fn begin_gated_dial(&mut self) {
        self.dialing = true;
        self.landing = Landing::new();
    }

    pub(crate) fn record_failure(&mut self, failure: DialFailure) {
        self.failures = self.failures.saturating_add(1);
        self.failure = Some(Recorded {
            seq: self.failures,
            failure,
        });
    }

    pub(crate) fn clear_failure(&mut self) {
        self.failure = None;
    }

    #[cfg(test)]
    pub(crate) fn has_failure(&self) -> bool {
        self.failure.is_some()
    }

    pub(crate) fn idle_total(&self) -> usize {
        let idle_h2 = self.h2.iter().filter(|e| e.state.is_trimmable()).count();
        self.h1.len().saturating_add(idle_h2)
    }
}
