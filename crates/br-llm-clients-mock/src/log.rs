use std::sync::Arc;
use std::task::Waker;

use br_llm_clients_core::{ClientRequest, ClientSettings};

use crate::call_log::{CallState, LoggedCall, Served, StateKind};
use crate::state_counts::StateCounts;
use crate::sync::Waiters;

pub(crate) struct LogState {
    began: usize,
    calls: Vec<Arc<LoggedCall>>,
    in_state: StateCounts,
    waiters: Waiters,
}

impl LogState {
    pub(crate) fn new() -> Self {
        Self {
            began: 0,
            calls: Vec::new(),
            in_state: StateCounts::default(),
            waiters: Waiters::default(),
        }
    }

    pub(crate) fn begin(
        &mut self,
        copies: Option<(ClientRequest, ClientSettings)>,
    ) -> (usize, Vec<Waker>) {
        let sequence = self.began;
        self.began = self.began.saturating_add(1);
        if let Some((request, settings)) = copies {
            self.calls
                .push(Arc::new(LoggedCall::new(sequence, request, settings)));
            self.in_state.enter(StateKind::Running);
        }
        (sequence, self.waiters.take())
    }

    pub(crate) fn began(&self) -> usize {
        self.began
    }

    pub(crate) fn set_served(&mut self, sequence: usize, served: Served) {
        if let Some(call) = self.calls.get_mut(sequence) {
            Arc::make_mut(call).set_served(served);
        }
    }

    pub(crate) fn set_state(&mut self, sequence: usize, state: CallState) -> Vec<Waker> {
        let Some(call) = self.calls.get_mut(sequence) else {
            return Vec::new();
        };
        if call.state().is_final() {
            return Vec::new();
        }
        self.in_state.leave(call.state().kind());
        self.in_state.enter(state.kind());
        Arc::make_mut(call).replace_state(state);
        self.waiters.take()
    }

    pub(crate) fn count_in(&self, kind: StateKind) -> usize {
        self.in_state.get(kind)
    }

    pub(crate) fn snapshot(&self) -> Vec<Arc<LoggedCall>> {
        self.calls.clone()
    }

    pub(crate) fn waiters_mut(&mut self) -> &mut Waiters {
        &mut self.waiters
    }
}
