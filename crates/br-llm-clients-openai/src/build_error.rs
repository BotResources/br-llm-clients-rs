#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum OpenAiBuildError {
    #[error("invalid credential: {reason}")]
    InvalidCredential { reason: String },
    #[error("invalid organization: {reason}")]
    InvalidOrganization { reason: String },
    #[error("invalid project: {reason}")]
    InvalidProject { reason: String },
    #[error("invalid endpoint: {reason}")]
    InvalidEndpoint { reason: String },
    #[error("invalid extra body: {reason}")]
    InvalidExtraBody { reason: String },
}
