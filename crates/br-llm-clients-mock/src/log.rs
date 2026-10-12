use std::task::Waker;

use br_llm_clients_core::{ClientRequest, ClientSettings};

use crate::call_log::{CallState, LoggedCall, Served, StateKind};
use crate::sync::Waiters;

pub(crate) struct LogState {
    began: usize,
    calls: Vec<LoggedCall>,
    in_state: [usize; StateKind::COUNT],
    waiters: Waiters,
}

impl LogState {
    pub(crate) fn new() -> Self {
        Self {
            began: 0,
            calls: Vec::new(),
            in_state: [0; StateKind::COUNT],
            waiters: Waiters::default(),
        }
    }

    pub(crate) fn begin(
        &mut self,
        copies: Option<(ClientRequest, ClientSettings)>,
    ) -> (usize, Vec<Waker>) {
        let sequence = self.began;
        self.began += 1;
        if let Some((request, settings)) = copies {
            self.calls
                .push(LoggedCall::new(sequence, request, settings));
            self.in_state[StateKind::Running.slot()] += 1;
        }
        (sequence, self.waiters.take())
    }

    pub(crate) fn began(&self) -> usize {
        self.began
    }

    pub(crate) fn set_served(&mut self, sequence: usize, served: Served) {
        if let Some(call) = self.calls.get_mut(sequence) {
            call.set_served(served);
        }
    }

    pub(crate) fn set_state(&mut self, sequence: usize, state: CallState) -> Vec<Waker> {
        let Some(call) = self.calls.get_mut(sequence) else {
            return Vec::new();
        };
        if call.state().is_final() {
            return Vec::new();
        }
        self.in_state[call.state().kind().slot()] -= 1;
        self.in_state[state.kind().slot()] += 1;
        call.replace_state(state);
        self.waiters.take()
    }

    pub(crate) fn count_in(&self, kind: StateKind) -> usize {
        self.in_state[kind.slot()]
    }

    pub(crate) fn snapshot(&self) -> Vec<LoggedCall> {
        self.calls.clone()
    }

    pub(crate) fn register(&mut self, slot: &mut Option<u64>, waker: &Waker) {
        self.waiters.register(slot, waker);
    }

    pub(crate) fn cancel(&mut self, slot: Option<u64>) {
        self.waiters.cancel(slot);
    }
}
