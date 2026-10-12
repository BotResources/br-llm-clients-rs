#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("the events do not fold into a step: {reason}")]
pub struct FoldError {
    reason: String,
}

impl FoldError {
    pub(crate) fn new(reason: String) -> Self {
        Self { reason }
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}
