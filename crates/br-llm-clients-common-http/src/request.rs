use std::fmt;

use bytes::Bytes;
use http::{HeaderName, HeaderValue};

use crate::config_error::ConfigError;
use crate::endpoint::Endpoint;
use crate::header::is_reserved;

#[derive(Clone)]
pub struct Request {
    endpoint: Endpoint,
    headers: Vec<(HeaderName, HeaderValue)>,
    body: Bytes,
}

impl Request {
    pub fn new(endpoint: Endpoint, body: impl Into<Vec<u8>>) -> Self {
        Self {
            endpoint,
            headers: Vec::new(),
            body: Bytes::from(body.into()),
        }
    }

    pub fn with_header(mut self, name: &str, value: &str) -> Result<Self, ConfigError> {
        let parsed =
            HeaderName::from_bytes(name.as_bytes()).map_err(|_| ConfigError::InvalidHeader {
                text: "the header name is not a valid header name".to_owned(),
            })?;
        if is_reserved(&parsed) {
            return Err(ConfigError::ReservedHeader {
                text: format!("`{}` is managed by the library", parsed.as_str()),
            });
        }
        let value = HeaderValue::from_str(value).map_err(|_| ConfigError::InvalidHeader {
            text: "the header value is not a valid header value".to_owned(),
        })?;
        if self.headers.iter().any(|(existing, _)| *existing == parsed) {
            return Err(ConfigError::DuplicateHeader {
                text: "a header name is given twice".to_owned(),
            });
        }
        self.headers.push((parsed, value));
        Ok(self)
    }

    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    pub fn header_names(&self) -> impl Iterator<Item = &str> {
        self.headers.iter().map(|(name, _)| name.as_str())
    }

    pub fn body_len(&self) -> usize {
        self.body.len()
    }

    pub(crate) fn into_parts(self) -> (Endpoint, Vec<(HeaderName, HeaderValue)>, Bytes) {
        (self.endpoint, self.headers, self.body)
    }
}

impl fmt::Debug for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names: Vec<&str> = self.header_names().collect();
        f.debug_struct("Request")
            .field("endpoint", &self.endpoint)
            .field("headers", &names)
            .field("body_len", &self.body.len())
            .finish()
    }
}
