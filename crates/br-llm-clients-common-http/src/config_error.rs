#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ConfigError {
    #[error("invalid endpoint: {text}")]
    InvalidEndpoint { text: String },
    #[error("invalid header: {text}")]
    InvalidHeader { text: String },
    #[error("duplicate header: {text}")]
    DuplicateHeader { text: String },
    #[error("reserved header: {text}")]
    ReservedHeader { text: String },
    #[error("invalid configuration: {text}")]
    InvalidConfig { text: String },
    #[error("empty secret: {text}")]
    EmptySecret { text: String },
}

impl ConfigError {
    pub(crate) fn endpoint(text: impl Into<String>) -> Self {
        Self::InvalidEndpoint { text: text.into() }
    }

    pub(crate) fn config(text: impl Into<String>) -> Self {
        Self::InvalidConfig { text: text.into() }
    }
}
