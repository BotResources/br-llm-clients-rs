use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DecodeError {
    InvalidLimit,
    FrameTooLarge { limit: usize },
    InvalidUtf8,
    InputTruncated,
    InputEnded,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimit => f.write_str("the limit of held bytes is zero"),
            Self::FrameTooLarge { limit } => {
                write!(
                    f,
                    "the bytes held by the decoder passed the limit of {limit}"
                )
            }
            Self::InvalidUtf8 => f.write_str("a data line or an event line is not valid UTF-8"),
            Self::InputTruncated => f.write_str("the input ended inside a frame or inside a line"),
            Self::InputEnded => f.write_str("the input has already ended"),
        }
    }
}

impl std::error::Error for DecodeError {}
