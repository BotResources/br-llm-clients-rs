use std::time::Duration;

use tokio::sync::watch;
use tokio::time::Instant;

use crate::deadline;

pub(crate) struct IdleClock {
    tx: watch::Sender<Option<Instant>>,
    limit: Duration,
    #[cfg(test)]
    before_arm: std::sync::Mutex<Option<Box<dyn FnOnce() + Send>>>,
}

impl IdleClock {
    pub(crate) fn new(limit: Duration) -> (Self, watch::Receiver<Option<Instant>>) {
        let (tx, rx) = watch::channel(None);
        (
            Self {
                tx,
                limit,
                #[cfg(test)]
                before_arm: std::sync::Mutex::new(None),
            },
            rx,
        )
    }

    pub(crate) fn arm(&self) {
        #[cfg(test)]
        self.run_before_arm();
        self.tx
            .send_replace(Some(deadline::after(Instant::now(), self.limit)));
    }

    pub(crate) fn disarm(&self) {
        self.tx.send_replace(None);
    }
}

#[cfg(test)]
impl IdleClock {
    pub(crate) fn is_armed(&self) -> bool {
        self.tx.borrow().is_some()
    }

    pub(crate) fn before_next_arm(&self, hook: impl FnOnce() + Send + 'static) {
        *self
            .before_arm
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Box::new(hook));
    }

    fn run_before_arm(&self) {
        let hook = self
            .before_arm
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if let Some(hook) = hook {
            hook();
        }
    }
}
