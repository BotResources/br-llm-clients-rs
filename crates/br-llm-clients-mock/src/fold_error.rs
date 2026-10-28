#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("the deltas do not fold into a response: {reason}")]
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
