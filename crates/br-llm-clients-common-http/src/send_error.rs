use crate::authorizer_error::AuthorizerError;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SendError {
    #[error("the authorizer failed: {0}")]
    Authorizer(#[source] AuthorizerError),
    #[error("invalid authorization: {text}")]
    InvalidAuthorization { text: String },
    #[error("connection failed: {text}")]
    Connect { text: String },
    #[error("TLS failed: {text}")]
    Tls { text: String },
    #[error("connect timeout: {text}")]
    ConnectTimeout { text: String },
    #[error("headers timeout: {text}")]
    HeadersTimeout { text: String },
    #[error("total timeout: {text}")]
    TotalTimeout { text: String },
    #[error("interrupted: {text}")]
    Interrupted { text: String },
    #[error("protocol error: {text}")]
    Protocol { text: String },
}

impl From<AuthorizerError> for SendError {
    fn from(error: AuthorizerError) -> Self {
        Self::Authorizer(error)
    }
}
