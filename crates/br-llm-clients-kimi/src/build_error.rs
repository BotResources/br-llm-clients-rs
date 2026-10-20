#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum KimiBuildError {
    #[error("invalid credential: {reason}")]
    InvalidCredential { reason: String },
    #[error("invalid endpoint: {reason}")]
    InvalidEndpoint { reason: String },
    #[error("invalid dialect options: {reason}")]
    InvalidDialectOptions { reason: String },
}
