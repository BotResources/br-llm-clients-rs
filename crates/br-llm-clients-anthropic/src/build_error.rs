#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum AnthropicBuildError {
    #[error("invalid credential: {reason}")]
    InvalidCredential { reason: String },
    #[error("invalid endpoint: {reason}")]
    InvalidEndpoint { reason: String },
    #[error("invalid workspace id: {reason}")]
    InvalidWorkspaceId { reason: String },
}

impl AnthropicBuildError {
    pub(crate) fn credential(reason: &str) -> Self {
        Self::InvalidCredential {
            reason: reason.to_owned(),
        }
    }

    pub(crate) fn endpoint(reason: &str) -> Self {
        Self::InvalidEndpoint {
            reason: reason.to_owned(),
        }
    }

    pub(crate) fn workspace_id(reason: &str) -> Self {
        Self::InvalidWorkspaceId {
            reason: reason.to_owned(),
        }
    }
}
