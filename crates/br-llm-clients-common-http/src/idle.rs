use std::time::Duration;

use tokio::sync::watch;
use tokio::time::Instant;

use crate::deadline;

pub(crate) struct IdleClock {
    tx: watch::Sender<Option<Instant>>,
    limit: Duration,
}

impl IdleClock {
    pub(crate) fn new(limit: Duration) -> (Self, watch::Receiver<Option<Instant>>) {
        let (tx, rx) = watch::channel(None);
        (Self { tx, limit }, rx)
    }

    pub(crate) fn arm(&self) {
        self.tx
            .send_replace(Some(deadline::after(Instant::now(), self.limit)));
    }

    pub(crate) fn disarm(&self) {
        self.tx.send_replace(None);
    }
}
