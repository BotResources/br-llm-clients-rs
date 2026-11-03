#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum OmlxBuildError {
    #[error("invalid credential: {reason}")]
    InvalidCredential { reason: String },
    #[error("invalid extra header: {reason}")]
    InvalidExtraHeader { reason: String },
    #[error("invalid endpoint: {reason}")]
    InvalidEndpoint { reason: String },
    #[error("invalid dialect options: {reason}")]
    InvalidDialectOptions { reason: String },
}
