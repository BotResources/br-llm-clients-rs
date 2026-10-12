use std::sync::Arc;
use std::task::Waker;

use br_llm_clients_core::CallOutcome;

use crate::call_log::{CallState, Served};
use crate::shared::Shared;
use crate::sync::wake_all;

#[derive(Clone)]
pub(crate) struct CallTracker {
    shared: Arc<Shared>,
    sequence: usize,
}

impl CallTracker {
    pub(crate) fn new(shared: Arc<Shared>, sequence: usize) -> Self {
        Self { shared, sequence }
    }

    pub(crate) fn sequence(&self) -> usize {
        self.sequence
    }

    pub(crate) fn shared(&self) -> &Arc<Shared> {
        &self.shared
    }

    pub(crate) fn set(&self, state: CallState) -> Vec<Waker> {
        if !self.shared.log_enabled() {
            return Vec::new();
        }
        self.shared
            .with(|inner| inner.log.set_state(self.sequence, state))
    }

    pub(crate) fn announce(&self, state: CallState) {
        wake_all(self.set(state));
    }

    pub(crate) fn serve(&self, served: Served) {
        self.shared
            .with(|inner| inner.log.set_served(self.sequence, served));
    }

    pub(crate) fn finish(&self, outcome: &CallOutcome) {
        if !self.shared.log_enabled() {
            return;
        }
        let state = match outcome.result() {
            Ok(_) => CallState::Succeeded(outcome.clone()),
            Err(_) => CallState::Failed(outcome.clone()),
        };
        self.announce(state);
    }
}

pub(crate) struct DropGuard {
    tracker: CallTracker,
}

impl DropGuard {
    pub(crate) fn new(tracker: CallTracker) -> Self {
        Self { tracker }
    }
}

impl Drop for DropGuard {
    fn drop(&mut self) {
        self.tracker.announce(CallState::Dropped);
    }
}
