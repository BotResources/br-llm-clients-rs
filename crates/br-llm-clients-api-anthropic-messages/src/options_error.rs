use std::fmt;

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum OptionsError {
    ReservedExtraBodyMember(String),
}

impl fmt::Display for OptionsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReservedExtraBodyMember(member) => {
                write!(f, "the extra body member `{member}` is reserved")
            }
        }
    }
}

impl std::error::Error for OptionsError {}
