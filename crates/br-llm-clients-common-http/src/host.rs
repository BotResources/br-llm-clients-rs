use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tokio::sync::watch;
use tokio::time::Instant;

use crate::claim::Landing;
use crate::h2state::H2State;
use crate::limits::{MAX_H2_PER_HOST, MAX_IDLE_PER_HOST};
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
        self.waiting.load(Ordering::SeqCst) > 0
            || self.dials.load(Ordering::SeqCst) > 0
            || !self.h2.is_empty()
            || !self.h1.is_empty()
            || self.live_h1.load(Ordering::SeqCst) > 0
    }

    pub(crate) fn drop_ended_h2(&mut self) {
        self.h2.retain(|entry| {
            if entry.sender.is_closed() {
                entry.state.end();
            }
            !entry.state.is_retired()
        });
    }

    pub(crate) fn reclaim_h2_room(&mut self) -> bool {
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

    #[cfg(test)]
    pub(crate) fn idle_total(&self) -> usize {
        self.idle_view().total()
    }

    fn idle_view(&self) -> IdleView {
        IdleView {
            h1: self
                .h1
                .iter()
                .enumerate()
                .filter(|(_, idle)| !idle.conn.sender.is_closed())
                .map(|(index, _)| index)
                .collect(),
            h2: self
                .h2
                .iter()
                .enumerate()
                .filter(|(_, entry)| entry.state.is_trimmable())
                .map(|(index, _)| index)
                .collect(),
        }
    }

    pub(crate) fn trim_to_cap(&mut self) -> bool {
        let mut trimmed_h2 = false;
        while let Some(victim) = self.oldest_surplus() {
            match victim {
                Oldest::H1(index) => {
                    self.h1.remove(index);
                }
                Oldest::H2(index) => {
                    self.trim_h2(index);
                    trimmed_h2 = true;
                }
            }
        }
        trimmed_h2
    }

    fn oldest_surplus(&self) -> Option<Oldest> {
        let view = self.idle_view();
        if view.total() <= MAX_IDLE_PER_HOST {
            return None;
        }
        let h1 = view.h1.first().copied();
        let h2 = view.h2.first().copied();
        match (h1, h2) {
            (Some(h1), Some(h2)) => {
                let since_h1 = self
                    .h1
                    .get(h1)
                    .and_then(|idle| idle.conn.idle.armed_until());
                let since_h2 = self.h2.get(h2).and_then(|entry| entry.state.idle_since());
                Some(if since_h1 <= since_h2 {
                    Oldest::H1(h1)
                } else {
                    Oldest::H2(h2)
                })
            }
            (Some(h1), None) => Some(Oldest::H1(h1)),
            (None, Some(h2)) => Some(Oldest::H2(h2)),
            (None, None) => None,
        }
    }
}

enum Oldest {
    H1(usize),
    H2(usize),
}

struct IdleView {
    h1: Vec<usize>,
    h2: Vec<usize>,
}

impl IdleView {
    fn total(&self) -> usize {
        self.h1.len().saturating_add(self.h2.len())
    }
}
