use crate::session_id::SessionId;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LiteLlmCallOptions {
    session_id: Option<SessionId>,
}

impl LiteLlmCallOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_session_id(mut self, session_id: SessionId) -> Self {
        self.session_id = Some(session_id);
        self
    }

    pub fn session_id(&self) -> Option<&SessionId> {
        self.session_id.as_ref()
    }
}
