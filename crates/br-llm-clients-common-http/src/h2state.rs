use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use tokio::sync::watch;

use crate::idle::IdleClock;
use crate::pool::HostHandle;
use crate::transport::bump;

pub(crate) struct H2State {
    inflight: Mutex<usize>,
    peer_max: AtomicUsize,
    known: watch::Sender<bool>,
    retired: AtomicBool,
    violation: Mutex<Option<String>>,
    silent: AtomicBool,
    gone: watch::Sender<bool>,
    idle: IdleClock,
    host: HostHandle,
}

impl H2State {
    pub(crate) fn new(idle: IdleClock, host: HostHandle) -> Arc<Self> {
        Arc::new(Self {
            inflight: Mutex::new(0),
            peer_max: AtomicUsize::new(0),
            known: watch::channel(false).0,
            retired: AtomicBool::new(false),
            violation: Mutex::new(None),
            silent: AtomicBool::new(false),
            gone: watch::channel(false).0,
            idle,
            host,
        })
    }

    fn count(&self) -> MutexGuard<'_, usize> {
        self.inflight.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn apply_settings(&self, max_concurrent_streams: Option<u32>) {
        let first = self.awaiting_settings();
        let limit = match max_concurrent_streams {
            Some(value) => usize::try_from(value).unwrap_or(usize::MAX),
            None if first => usize::MAX,
            None => return,
        };
        self.peer_max.store(limit, Ordering::SeqCst);
        self.known.send_replace(true);
        self.wake_host();
    }

    pub(crate) fn awaiting_settings(&self) -> bool {
        !*self.known.borrow()
    }

    pub(crate) fn record_violation(&self, text: String) {
        *self
            .violation
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(text);
    }

    pub(crate) fn mark_silent(&self) {
        self.silent.store(true, Ordering::SeqCst);
    }

    pub(crate) fn is_silent(&self) -> bool {
        self.silent.load(Ordering::SeqCst)
    }

    pub(crate) fn violation(&self) -> Option<String> {
        self.violation
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub(crate) fn mark_gone(&self) {
        self.gone.send_replace(true);
    }

    pub(crate) fn retire(&self) {
        self.retired.store(true, Ordering::SeqCst);
    }

    pub(crate) fn limit_unknown(&self) -> bool {
        self.awaiting_settings() && !self.is_retired()
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
        if self.awaiting_settings() || *count >= self.peer_max.load(Ordering::SeqCst) {
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
    pub(crate) fn gone(&self) -> watch::Receiver<bool> {
        self.0.gone.subscribe()
    }

    pub(crate) fn retire_connection(&self) {
        self.0.retire();
    }

    pub(crate) fn violation(&self) -> Option<String> {
        self.0.violation()
    }

    pub(crate) async fn announced_limit(&self) -> usize {
        let mut known = self.0.known.subscribe();
        let _ = known.wait_for(|known| *known).await;
        self.0.peer_max.load(Ordering::SeqCst)
    }
}

impl Drop for Permit {
    fn drop(&mut self) {
        let became_idle = {
            let mut count = self.0.count();
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.0.idle.arm();
            }
            *count == 0
        };
        self.0.wake_host();
        if let Some(pool) = self.0.host.pool.upgrade().filter(|_| became_idle) {
            pool.trim_idle(&self.0.host.key);
        }
    }
}

#[cfg(test)]
impl H2State {
    pub(crate) fn before_next_arm(&self, hook: impl FnOnce() + Send + 'static) {
        self.idle.before_next_arm(hook);
    }

    pub(crate) fn violates_idle_invariant(&self) -> bool {
        let count = self.count();
        *count > 0 && self.idle.is_armed()
    }
}
