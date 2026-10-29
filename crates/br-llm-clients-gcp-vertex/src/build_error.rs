#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum VertexBuildError {
    #[error("invalid project: {reason}")]
    InvalidProject { reason: String },
    #[error("invalid location: {reason}")]
    InvalidLocation { reason: String },
    #[error("invalid credential: {reason}")]
    InvalidCredential { reason: String },
    #[error("invalid endpoint: {reason}")]
    InvalidEndpoint { reason: String },
}

impl VertexBuildError {
    pub(crate) fn project(reason: impl Into<String>) -> Self {
        Self::InvalidProject {
            reason: reason.into(),
        }
    }

    pub(crate) fn location(reason: impl Into<String>) -> Self {
        Self::InvalidLocation {
            reason: reason.into(),
        }
    }

    pub(crate) fn credential(reason: impl Into<String>) -> Self {
        Self::InvalidCredential {
            reason: reason.into(),
        }
    }

    pub(crate) fn endpoint(reason: impl Into<String>) -> Self {
        Self::InvalidEndpoint {
            reason: reason.into(),
        }
    }
}
