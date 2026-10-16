use std::sync::atomic::{AtomicU8, Ordering};

const LIVE: u8 = 0;
const ENDED: u8 = 1;
const TRIMMED: u8 = 2;

pub(crate) struct Retirement(AtomicU8);

impl Retirement {
    pub(crate) fn new() -> Self {
        Self(AtomicU8::new(LIVE))
    }

    pub(crate) fn end(&self) {
        self.leave(ENDED);
    }

    pub(crate) fn trim(&self) {
        self.leave(TRIMMED);
    }

    pub(crate) fn is_retired(&self) -> bool {
        self.0.load(Ordering::SeqCst) != LIVE
    }

    pub(crate) fn is_ended(&self) -> bool {
        self.0.load(Ordering::SeqCst) == ENDED
    }

    fn leave(&self, reason: u8) {
        let _ = self
            .0
            .compare_exchange(LIVE, reason, Ordering::SeqCst, Ordering::SeqCst);
    }
}

#[cfg(test)]
#[path = "retirement_tests.rs"]
mod tests;
