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

    fn trim_h2(&mut self, index: usize) {
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
        self.counted().count()
    }

    fn counted(&self) -> impl Iterator<Item = (Option<Instant>, Oldest)> + '_ {
        let h1 = self
            .h1
            .iter()
            .enumerate()
            .filter(|(_, idle)| !idle.conn.sender.is_closed())
            .map(|(index, idle)| (idle.conn.idle.armed_until(), Oldest::H1(index)));
        let h2 = self
            .h2
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.counts())
            .map(|(index, entry)| (entry.state.idle_since(), Oldest::H2(index)));
        h1.chain(h2)
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
        let mut counted = 0usize;
        let oldest = self
            .counted()
            .inspect(|_| counted = counted.saturating_add(1))
            .min_by_key(|(idle_until, _)| *idle_until);
        if counted <= MAX_IDLE_PER_HOST {
            return None;
        }
        oldest.map(|(_, which)| which)
    }
}

impl H2Entry {
    fn counts(&self) -> bool {
        !self.sender.is_closed() && self.state.is_trimmable()
    }
}

enum Oldest {
    H1(usize),
    H2(usize),
}
