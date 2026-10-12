use br_llm_clients_core::{CallOutcome, ClientError, ClientRequest, ClientSettings};

#[derive(Debug, Clone, PartialEq)]
pub enum Served {
    NotYetClaimed,
    Rule { position: usize },
    Entry { position: usize },
    Refusal(ClientError),
    Fallback,
    Exhausted,
    Nothing,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CallState {
    Running,
    WaitingAtGate,
    Hanging,
    Succeeded(CallOutcome),
    Failed(CallOutcome),
    Dropped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StateKind {
    Running,
    WaitingAtGate,
    Hanging,
    Succeeded,
    Failed,
    Dropped,
}

impl CallState {
    pub fn kind(&self) -> StateKind {
        match self {
            CallState::Running => StateKind::Running,
            CallState::WaitingAtGate => StateKind::WaitingAtGate,
            CallState::Hanging => StateKind::Hanging,
            CallState::Succeeded(_) => StateKind::Succeeded,
            CallState::Failed(_) => StateKind::Failed,
            CallState::Dropped => StateKind::Dropped,
        }
    }

    pub(crate) fn is_final(&self) -> bool {
        matches!(
            self,
            CallState::Succeeded(_) | CallState::Failed(_) | CallState::Dropped
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoggedCall {
    sequence: usize,
    request: ClientRequest,
    settings: ClientSettings,
    served: Served,
    state: CallState,
}

impl LoggedCall {
    pub(crate) fn new(sequence: usize, request: ClientRequest, settings: ClientSettings) -> Self {
        Self {
            sequence,
            request,
            settings,
            served: Served::NotYetClaimed,
            state: CallState::Running,
        }
    }

    pub fn sequence(&self) -> usize {
        self.sequence
    }

    pub fn request(&self) -> &ClientRequest {
        &self.request
    }

    pub fn settings(&self) -> &ClientSettings {
        &self.settings
    }

    pub fn served(&self) -> &Served {
        &self.served
    }

    pub fn state(&self) -> &CallState {
        &self.state
    }

    pub(crate) fn set_served(&mut self, served: Served) {
        self.served = served;
    }

    pub(crate) fn replace_state(&mut self, state: CallState) {
        self.state = state;
    }
}
