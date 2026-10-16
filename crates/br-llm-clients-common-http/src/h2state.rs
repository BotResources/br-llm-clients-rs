use std::future::pending;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use tokio::sync::watch;
use tokio::time::Instant;

use crate::announced::Announced;
use crate::idle::IdleClock;
use crate::pool::HostHandle;
use crate::retirement::Retirement;
use crate::settled::Settled;
use crate::transport::bump;

pub(crate) struct H2State {
    inflight: Mutex<usize>,
    announced: watch::Sender<Announced>,
    retirement: Retirement,
    violation: Mutex<Option<String>>,
    gone: watch::Sender<bool>,
    idle: IdleClock,
    host: HostHandle,
}

impl H2State {
    pub(crate) fn new(idle: IdleClock, host: HostHandle) -> Arc<Self> {
        Arc::new(Self {
            inflight: Mutex::new(0),
            announced: watch::channel(Announced::Awaiting).0,
            retirement: Retirement::new(),
            violation: Mutex::new(None),
            gone: watch::channel(false).0,
            idle,
            host,
        })
    }

    fn count(&self) -> MutexGuard<'_, usize> {
        self.inflight.lock().unwrap_or_else(PoisonError::into_inner)
    }

    #[cfg(test)]
    pub(crate) fn apply_settings(&self, max_concurrent_streams: Option<u32>) {
        self.apply_settings_run(&[max_concurrent_streams]);
    }

    pub(crate) fn apply_settings_run(&self, run: &[Option<u32>]) {
        let Some(next) = self.announcement().after_run(run) else {
            return;
        };
        #[cfg(test)]
        self.step();
        self.announced.send_replace(next);
        #[cfg(test)]
        self.step();
        self.wake_host();
        #[cfg(test)]
        self.step();
    }

    pub(crate) fn announce_run(&self, run: &[Option<u32>]) {
        self.apply_settings_run(run);
        self.trim_host();
    }

    fn announcement(&self) -> Announced {
        *self.announced.borrow()
    }

    pub(crate) fn awaiting_settings(&self) -> bool {
        self.announcement().is_awaiting()
    }

    pub(crate) fn record_violation(&self, text: String) {
        *self
            .violation
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(text);
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

    pub(crate) fn end(&self) {
        #[cfg(test)]
        self.step_end();
        self.retirement.end();
    }

    pub(crate) fn trim(&self) {
        self.retirement.trim();
    }

    pub(crate) fn limit_unknown(&self) -> bool {
        self.awaiting_settings()
    }

    pub(crate) fn is_retired(&self) -> bool {
        self.retirement.is_retired()
    }

    pub(crate) fn is_idle(&self) -> bool {
        *self.count() == 0
    }

    pub(crate) fn is_trimmable(&self) -> bool {
        !self.is_retired()
            && self.is_idle()
            && matches!(self.announcement(), Announced::Usable(limit) if limit > 0)
    }

    pub(crate) fn idle_since(&self) -> Option<Instant> {
        self.idle.armed_until()
    }

    pub(crate) fn settled(&self) -> Option<Settled> {
        let ended = self.retirement.is_ended() || self.announcement().is_zero_since_first();
        #[cfg(test)]
        self.step_settled();
        if let Some(text) = self.violation() {
            return Some(Settled::Broken(text));
        }
        ended.then_some(Settled::Spent)
    }

    pub(crate) fn wake_host(&self) {
        bump(&self.host.signal);
    }

    fn trim_host(&self) {
        if let Some(pool) = self.host.pool.upgrade() {
            pool.trim_idle(&self.host.key);
        }
    }

    pub(crate) fn arm_idle(&self) {
        self.idle.arm();
    }

    pub(crate) fn try_acquire(self: &Arc<Self>) -> Option<Permit> {
        if self.is_retired() {
            return None;
        }
        let mut count = self.count();
        if *count >= self.announcement().limit() {
            return None;
        }
        if !self.idle.disarm() {
            return None;
        }
        *count = count.saturating_add(1);
        Some(Permit(Arc::clone(self)))
    }

    pub(crate) fn force_acquire(self: &Arc<Self>) -> Permit {
        let mut count = self.count();
        *count = count.saturating_add(1);
        if !self.idle.disarm() {
            self.end();
        }
        Permit(Arc::clone(self))
    }
}

pub(crate) struct Permit(Arc<H2State>);

impl Permit {
    pub(crate) fn gone(&self) -> watch::Receiver<bool> {
        self.0.gone.subscribe()
    }

    pub(crate) fn end_connection(&self) {
        self.0.end();
    }

    pub(crate) fn violation(&self) -> Option<String> {
        self.0.violation()
    }

    pub(crate) async fn usable_since_first_settings(&self) -> bool {
        let mut announced = self.0.announced.subscribe();
        let Ok(settled) = announced
            .wait_for(|announced| !announced.is_awaiting())
            .await
            .map(|announced| *announced)
        else {
            return pending().await;
        };
        !settled.is_zero_since_first()
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
        if became_idle {
            self.0.trim_host();
        }
    }
}

#[cfg(test)]
impl H2State {
    fn step(&self) {
        if let Some(pool) = self.host.pool.upgrade() {
            pool.hooks.run_settings_step();
        }
    }

    fn step_end(&self) {
        if let Some(pool) = self.host.pool.upgrade() {
            pool.hooks.run_end_step();
        }
    }

    fn step_settled(&self) {
        if let Some(pool) = self.host.pool.upgrade() {
            pool.hooks.run_settled_step();
        }
    }

    pub(crate) fn before_next_arm(&self, hook: impl FnOnce() + Send + 'static) {
        self.idle.before_next_arm(hook);
    }

    pub(crate) fn violates_idle_invariant(&self) -> bool {
        let count = self.count();
        *count > 0 && self.idle.is_armed()
    }
}
