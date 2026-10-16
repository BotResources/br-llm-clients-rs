use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tokio::sync::watch;
use tokio::time::Instant;

use crate::claim::Landing;
use crate::h2state::H2State;
use crate::pool::MAX_H2_PER_HOST;
use crate::transport::{H1Conn, H2Sender, LiveGuard, Signal};

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

pub(crate) struct Host {
    pub(crate) proto: Option<Proto>,
    pub(crate) dialing: bool,
    pub(crate) landing: Arc<Landing>,
    pub(crate) h2: Vec<H2Entry>,
    pub(crate) h1: Vec<Idle>,
    pub(crate) live_h1: Arc<AtomicUsize>,
    pub(crate) waiting: Arc<AtomicUsize>,
    dials: Arc<AtomicUsize>,
    pub(crate) signal: Signal,
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
            dials: Arc::new(AtomicUsize::new(0)),
            signal: Arc::new(watch::channel(0u64).0),
        }
    }

    pub(crate) fn in_use(&self) -> bool {
        self.dialing
            || self.waiting.load(Ordering::SeqCst) > 0
            || self.dials.load(Ordering::SeqCst) > 0
            || !self.h2.is_empty()
            || !self.h1.is_empty()
            || self.live_h1.load(Ordering::SeqCst) > 0
    }

    pub(crate) fn drop_ended_h2(&mut self) {
        self.h2.retain(|entry| {
            if entry.sender.is_closed() {
                entry.state.retire();
            }
            !entry.state.is_retired()
        });
    }

    pub(crate) fn has_h2_room(&mut self) -> bool {
        self.drop_ended_h2();
        self.h2.len() < MAX_H2_PER_HOST
    }

    pub(crate) fn pin_for_dial(&self) -> LiveGuard {
        LiveGuard::new(&self.dials)
    }

    pub(crate) fn trim_h2(&mut self, index: usize) {
        let Some(entry) = self.h2.get(index) else {
            return;
        };
        entry.state.trim();
        self.h2.remove(index);
    }

    pub(crate) fn begin_gated_dial(&mut self) {
        self.dialing = true;
        self.landing = Landing::new();
    }

    pub(crate) fn idle_total(&self) -> usize {
        let idle_h2 = self.h2.iter().filter(|e| e.state.is_trimmable()).count();
        self.h1.len().saturating_add(idle_h2)
    }
}
