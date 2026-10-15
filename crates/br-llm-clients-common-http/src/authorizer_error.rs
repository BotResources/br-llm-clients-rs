#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum AuthorizerError {
    #[error("the authorizer has no usable credential: {text}")]
    Unavailable { text: String },
    #[error("the authorizer could not fetch a credential: {text}")]
    FetchFailed { text: String },
}

impl AuthorizerError {
    pub fn unavailable(text: impl Into<String>) -> Self {
        Self::Unavailable { text: text.into() }
    }

    pub fn fetch_failed(text: impl Into<String>) -> Self {
        Self::FetchFailed { text: text.into() }
    }
}

impl AuthorizerError {
    pub(crate) fn capped(self) -> Self {
        let cleaned = |text: String| crate::redact::Redactor::default().finish(&text);
        match self {
            Self::Unavailable { text } => Self::Unavailable {
                text: cleaned(text),
            },
            Self::FetchFailed { text } => Self::FetchFailed {
                text: cleaned(text),
            },
        }
    }
}
