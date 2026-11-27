use std::fmt;

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum OptionsError {
    ReservedExtraBodyMember { member: String },
    InvalidCacheOption { member: Option<String> },
}

impl fmt::Display for OptionsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReservedExtraBodyMember { member } => {
                write!(f, "the extra body member `{member}` is reserved")
            }
            Self::InvalidCacheOption {
                member: Some(member),
            } => write!(f, "the cache option member `{member}` is refused"),
            Self::InvalidCacheOption { member: None } => {
                write!(f, "the cache options are not a JSON object")
            }
        }
    }
}

impl std::error::Error for OptionsError {}
