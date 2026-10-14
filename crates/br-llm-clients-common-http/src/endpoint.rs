use std::fmt;

use http::Uri;

use crate::config_error::ConfigError;
use crate::endpoint_parse::parse;
use crate::endpoint_scheme::Scheme;
use crate::percent::{encode_query_component, encode_segment};

#[derive(Clone, PartialEq, Eq, Hash)]
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
        Ok(Self {
            path,
            ..self.clone()
        })
    }

    pub fn with_query_pair(&self, name: &str, value: &str) -> Self {
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
    }

    pub fn is_https(&self) -> bool {
        self.scheme == Scheme::Https
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> Option<u16> {
        self.port
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn path_and_query(&self) -> String {
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
