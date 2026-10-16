use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;
use tokio::time::Instant;

use crate::deadline;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Slot {
    Busy,
    Idle(Instant),
    Expired,
}

pub(crate) struct IdleClock {
    tx: Arc<watch::Sender<Slot>>,
    limit: Duration,
    #[cfg(test)]
    before_arm: std::sync::Mutex<Option<Box<dyn FnOnce() + Send>>>,
}

pub(crate) struct IdleWatch {
    tx: Arc<watch::Sender<Slot>>,
    rx: watch::Receiver<Slot>,
}

impl IdleClock {
    pub(crate) fn new(limit: Duration) -> (Self, IdleWatch) {
        let (tx, rx) = watch::channel(Slot::Busy);
        let tx = Arc::new(tx);
        let watch = IdleWatch {
            tx: Arc::clone(&tx),
            rx,
        };
        (
            Self {
                tx,
                limit,
                #[cfg(test)]
                before_arm: std::sync::Mutex::new(None),
            },
            watch,
        )
    }

    pub(crate) fn arm(&self) {
        #[cfg(test)]
        self.run_before_arm();
        let deadline = deadline::after(Instant::now(), self.limit);
        self.tx.send_if_modified(|slot| {
            let live = *slot != Slot::Expired;
            if live {
                *slot = Slot::Idle(deadline);
            }
            live
        });
    }

    pub(crate) fn armed_until(&self) -> Option<Instant> {
        match *self.tx.borrow() {
            Slot::Idle(at) => Some(at),
            Slot::Busy | Slot::Expired => None,
        }
    }

    pub(crate) fn disarm(&self) -> bool {
        let mut live = true;
        self.tx.send_if_modified(|slot| {
            live = *slot != Slot::Expired;
            if live {
                *slot = Slot::Busy;
            }
            live
        });
        live
    }
}

impl IdleWatch {
    pub(crate) fn armed_until(&mut self) -> Option<Instant> {
        match *self.rx.borrow_and_update() {
            Slot::Idle(at) => Some(at),
            Slot::Busy | Slot::Expired => None,
        }
    }

    pub(crate) async fn changed(&mut self) {
        let _ = self.rx.changed().await;
    }

    pub(crate) fn expire(&self, at: Instant) -> bool {
        self.tx.send_if_modified(|slot| {
            let due = *slot == Slot::Idle(at);
            if due {
                *slot = Slot::Expired;
            }
            due
        })
    }
}

#[cfg(test)]
impl IdleClock {
    pub(crate) fn is_armed(&self) -> bool {
        matches!(*self.tx.borrow(), Slot::Idle(_))
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
