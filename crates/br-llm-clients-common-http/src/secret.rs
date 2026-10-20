use std::fmt;

use crate::config_error::ConfigError;

const PLACEHOLDER: &str = "[redacted]";

#[derive(Clone)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Result<Self, ConfigError> {
        let value = value.into();
        if value.is_empty() {
            return Err(ConfigError::EmptySecret {
                text: "a secret cannot be empty".to_owned(),
            });
        }
        Ok(Self(value))
    }

    pub(crate) fn from_non_empty(value: String) -> Self {
        Self(value)
    }

    pub fn expose(&self) -> &str {
        &self.0
    }

    pub fn appears_in(&self, text: &str) -> bool {
        text.contains(self.0.as_str())
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(PLACEHOLDER)
    }
}

impl fmt::Display for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(PLACEHOLDER)
    }
}

pub(crate) const REDACTION: &str = PLACEHOLDER;
