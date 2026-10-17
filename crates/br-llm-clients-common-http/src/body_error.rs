#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum BodyError {
    #[error("interrupted: {text}")]
    Interrupted { text: String },
    #[error("idle timeout: {text}")]
    IdleTimeout { text: String },
    #[error("total timeout: {text}")]
    TotalTimeout { text: String },
    #[error("unsupported content encoding: {text}")]
    UnsupportedEncoding { text: String },
    #[error("protocol error: {text}")]
    Protocol { text: String },
}
