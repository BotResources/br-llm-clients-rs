#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the call log is switched off, so no wait on the state of calls is possible")]
pub struct LogDisabled;
