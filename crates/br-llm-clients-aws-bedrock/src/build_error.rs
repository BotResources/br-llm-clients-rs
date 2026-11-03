#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum BedrockBuildError {
    #[error("invalid region: {reason}")]
    InvalidRegion { reason: String },
    #[error("invalid credential: {reason}")]
    InvalidCredential { reason: String },
    #[error("invalid endpoint: {reason}")]
    InvalidEndpoint { reason: String },
}

impl BedrockBuildError {
    pub(crate) fn region(reason: impl Into<String>) -> Self {
        Self::InvalidRegion {
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
