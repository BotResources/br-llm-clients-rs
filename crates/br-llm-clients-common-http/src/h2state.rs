use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::idle::IdleClock;
use crate::pool::HostHandle;
use crate::transport::bump;

pub(crate) struct H2State {
    inflight: Mutex<usize>,
    peer_max: AtomicUsize,
    retired: AtomicBool,
    idle: IdleClock,
    host: HostHandle,
}

impl H2State {
    pub(crate) fn new(idle: IdleClock, host: HostHandle) -> Arc<Self> {
        Arc::new(Self {
            inflight: Mutex::new(0),
            peer_max: AtomicUsize::new(0),
            retired: AtomicBool::new(false),
            idle,
            host,
        })
    }

    fn count(&self) -> MutexGuard<'_, usize> {
        self.inflight.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn set_peer_max(&self, max: usize) -> bool {
        self.peer_max.swap(max, Ordering::SeqCst) != max
    }

    pub(crate) fn retire(&self) {
        self.retired.store(true, Ordering::SeqCst);
    }

    pub(crate) fn limit_unknown(&self) -> bool {
        self.peer_max.load(Ordering::SeqCst) == 0 && !self.is_retired()
    }

    pub(crate) fn is_retired(&self) -> bool {
        self.retired.load(Ordering::SeqCst)
    }

    pub(crate) fn is_idle(&self) -> bool {
        *self.count() == 0
    }

    pub(crate) fn wake_host(&self) {
        bump(&self.host.signal);
    }

    pub(crate) fn arm_idle(&self) {
        let count = self.count();
        if *count == 0 {
            self.idle.arm();
        }
    }

    pub(crate) fn try_acquire(self: &Arc<Self>) -> Option<Permit> {
        if self.is_retired() {
            return None;
        }
        let mut count = self.count();
        if *count >= self.peer_max.load(Ordering::SeqCst) {
            return None;
        }
        *count = count.saturating_add(1);
        self.idle.disarm();
        Some(Permit(Arc::clone(self)))
    }

    pub(crate) fn force_acquire(self: &Arc<Self>) -> Permit {
        let mut count = self.count();
        *count = count.saturating_add(1);
        self.idle.disarm();
        Permit(Arc::clone(self))
    }
}

pub(crate) struct Permit(Arc<H2State>);

impl Permit {
    pub(crate) fn retire_connection(&self) {
        self.0.retire();
    }
}

impl Drop for Permit {
    fn drop(&mut self) {
        let became_idle = {
            let mut count = self.0.count();
            *count = count.saturating_sub(1);
            *count == 0
        };
        if became_idle {
            self.0.idle.arm();
        }
        self.0.wake_host();
        if let Some(pool) = self.0.host.pool.upgrade().filter(|_| became_idle) {
            pool.trim_idle(&self.0.host.key);
        }
    }
}
