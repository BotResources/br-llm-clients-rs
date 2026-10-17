use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use hyper::client::conn::{http1, http2};
use tokio::sync::watch;
use tokio::task::AbortHandle;

use crate::h2state::Permit;
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

pub(crate) enum Conn {
    H1(H1Conn),
    H2 { sender: H2Sender, permit: Permit },
}
