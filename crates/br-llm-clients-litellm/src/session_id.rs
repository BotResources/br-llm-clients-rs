use std::fmt;

const MAX_BYTES: usize = 256;
const FIRST_ALLOWED: u8 = 0x20;
const LAST_ALLOWED: u8 = 0x7E;
const SPACE: u8 = b' ';

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(String);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SessionIdError {
    #[error("the session id is empty")]
    Empty,
    #[error("the session id is longer than {MAX_BYTES} bytes")]
    TooLong,
    #[error("the session id starts or ends with a space")]
    OuterSpace,
    #[error("the session id holds a byte outside 0x20 to 0x7E at index {index} (counted from 0)")]
    InvalidByte { index: usize },
}

impl SessionId {
    pub fn new(value: impl Into<String>) -> Result<Self, SessionIdError> {
        let value = value.into();
        let bytes = value.as_bytes();
        if bytes.is_empty() {
            return Err(SessionIdError::Empty);
        }
        if bytes.len() > MAX_BYTES {
            return Err(SessionIdError::TooLong);
        }
        if let Some(index) = bytes
            .iter()
            .position(|byte| !(FIRST_ALLOWED..=LAST_ALLOWED).contains(byte))
        {
            return Err(SessionIdError::InvalidByte { index });
        }
        if bytes.first() == Some(&SPACE) || bytes.last() == Some(&SPACE) {
            return Err(SessionIdError::OuterSpace);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
