use std::fmt;

use http::Uri;

use crate::config_error::ConfigError;
use crate::endpoint_parse::parse;
use crate::endpoint_scheme::Scheme;
use crate::percent::{encode_query_component, encode_segment};

const MAX_ENDPOINT_BYTES: usize = 8192;

fn too_long() -> ConfigError {
    ConfigError::endpoint("it is longer than 8192 bytes")
}

#[derive(Clone, PartialEq, Eq)]
pub struct Endpoint {
    scheme: Scheme,
    host: String,
    port: Option<u16>,
    path: String,
    query: Option<String>,
}

impl Endpoint {
    pub fn https(text: &str) -> Result<Self, ConfigError> {
        Self::from_text(text, Scheme::Https)
    }

    pub fn plain_http(text: &str) -> Result<Self, ConfigError> {
        Self::from_text(text, Scheme::Http)
    }

    fn from_text(text: &str, scheme: Scheme) -> Result<Self, ConfigError> {
        if text.len() > MAX_ENDPOINT_BYTES {
            return Err(too_long());
        }
        let parsed = parse(text, scheme)?;
        let endpoint = Self {
            scheme,
            host: parsed.host,
            port: parsed.port,
            path: parsed.path,
            query: parsed.query,
        };
        endpoint.absolute_uri()?;
        Ok(endpoint)
    }

    pub fn join_segment(&self, segment: &str) -> Result<Self, ConfigError> {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err(ConfigError::endpoint(
                "a path segment cannot be empty, `.` or `..`",
            ));
        }
        let mut path = self.path.clone();
        if !path.ends_with('/') {
            path.push('/');
        }
        path.push_str(&encode_segment(segment));
        Self {
            path,
            ..self.clone()
        }
        .checked_length()
    }

    pub fn with_query_pair(&self, name: &str, value: &str) -> Result<Self, ConfigError> {
        let mut query = self.query.clone().unwrap_or_default();
        if !query.is_empty() {
            query.push('&');
        }
        query.push_str(&encode_query_component(name));
        query.push('=');
        query.push_str(&encode_query_component(value));
        Self {
            query: Some(query),
            ..self.clone()
        }
        .checked_length()
    }

    fn checked_length(self) -> Result<Self, ConfigError> {
        let port = self
            .port
            .map_or(0, |port| port.to_string().len().saturating_add(1));
        let length = self
            .scheme
            .as_str()
            .len()
            .saturating_add(3)
            .saturating_add(self.host.len())
            .saturating_add(port)
            .saturating_add(self.path_and_query().len());
        if length > MAX_ENDPOINT_BYTES {
            return Err(too_long());
        }
        Ok(self)
    }

    pub(crate) fn is_https(&self) -> bool {
        self.scheme == Scheme::Https
    }

    #[cfg(test)]
    pub(crate) fn host(&self) -> &str {
        &self.host
    }

    #[cfg(test)]
    pub(crate) fn port(&self) -> Option<u16> {
        self.port
    }

    #[cfg(test)]
    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    pub(crate) fn path_and_query(&self) -> String {
        match &self.query {
            Some(query) => format!("{}?{}", self.path, query),
            None => self.path.clone(),
        }
    }

    pub(crate) fn effective_port(&self) -> u16 {
        self.port.unwrap_or_else(|| self.scheme.default_port())
    }

    pub(crate) fn host_header(&self) -> String {
        match self.port {
            Some(port) if port != self.scheme.default_port() => format!("{}:{}", self.host, port),
            _ => self.host.clone(),
        }
    }

    pub(crate) fn dns_name(&self) -> &str {
        self.host
            .strip_prefix('[')
            .and_then(|h| h.strip_suffix(']'))
            .unwrap_or(&self.host)
    }

    pub(crate) fn absolute_uri(&self) -> Result<Uri, ConfigError> {
        let text = format!(
            "{}://{}{}",
            self.scheme.as_str(),
            self.host_header(),
            self.path_and_query()
        );
        text.parse::<Uri>()
            .map_err(|_| ConfigError::endpoint("it is not a valid URL"))
    }

    pub(crate) fn origin_uri(&self) -> Result<Uri, ConfigError> {
        self.path_and_query()
            .parse::<Uri>()
            .map_err(|_| ConfigError::endpoint("it is not a valid URL"))
    }
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}://{}", self.scheme.as_str(), self.host)?;
        if let Some(port) = self.port {
            write!(f, ":{port}")?;
        }
        f.write_str(&self.path)
    }
}

impl fmt::Debug for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Endpoint")
            .field("scheme", &self.scheme.as_str())
            .field("host", &self.host)
            .field("port", &self.port)
            .field("path", &self.path)
            .finish()
    }
}
