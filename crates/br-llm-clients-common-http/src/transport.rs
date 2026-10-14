use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use hyper::client::conn::{http1, http2};
use tokio::sync::watch;
use tokio::task::AbortHandle;

use crate::idle::IdleClock;
use crate::payload::Payload;

pub(crate) type H1Sender = http1::SendRequest<Payload>;
pub(crate) type H2Sender = http2::SendRequest<Payload>;
pub(crate) type Signal = Arc<watch::Sender<u64>>;

pub(crate) fn bump(signal: &Signal) {
    signal.send_modify(|version| *version = version.wrapping_add(1));
}

pub(crate) struct LiveGuard(Arc<AtomicUsize>);

impl LiveGuard {
    pub(crate) fn new(counter: &Arc<AtomicUsize>) -> Self {
        counter.fetch_add(1, Ordering::SeqCst);
        Self(Arc::clone(counter))
    }
}

impl Drop for LiveGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

pub(crate) struct H1Conn {
    pub(crate) sender: H1Sender,
    pub(crate) idle: IdleClock,
    driver: AbortHandle,
    live: Option<LiveGuard>,
}

impl H1Conn {
    pub(crate) fn new(sender: H1Sender, idle: IdleClock, driver: AbortHandle) -> Self {
        Self {
            sender,
            idle,
            driver,
            live: None,
        }
    }

    pub(crate) fn count_as_live(&mut self, counter: &Arc<AtomicUsize>) {
        self.live = Some(LiveGuard::new(counter));
    }
}

impl Drop for H1Conn {
    fn drop(&mut self) {
        self.driver.abort();
    }
}

pub(crate) struct H2State {
    inflight: Mutex<usize>,
    peer_max: AtomicUsize,
    idle: IdleClock,
    signal: Signal,
}

impl H2State {
    pub(crate) fn new(idle: IdleClock, signal: Signal) -> Arc<Self> {
        Arc::new(Self {
            inflight: Mutex::new(0),
            peer_max: AtomicUsize::new(usize::MAX),
            idle,
            signal,
        })
    }

    fn count(&self) -> MutexGuard<'_, usize> {
        self.inflight.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn set_max_streams(&self, max: usize) {
        self.peer_max.store(max.max(1), Ordering::SeqCst);
    }

    pub(crate) fn arm_idle(&self) {
        let count = self.count();
        if *count == 0 {
            self.idle.arm();
        }
    }

    pub(crate) fn try_acquire(self: &Arc<Self>) -> Option<Permit> {
        let mut count = self.count();
        if *count >= self.peer_max.load(Ordering::SeqCst) {
            return None;
        }
        *count = count.saturating_add(1);
        self.idle.disarm();
        Some(Permit(Arc::clone(self)))
    }
}

pub(crate) struct Permit(Arc<H2State>);

impl Drop for Permit {
    fn drop(&mut self) {
        {
            let mut count = self.0.count();
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.0.idle.arm();
            }
        }
        bump(&self.0.signal);
    }
}

pub(crate) enum Conn {
    H1(H1Conn),
    H2 { sender: H2Sender, permit: Permit },
}
