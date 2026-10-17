use std::time::Duration;

use http::HeaderValue;

use crate::config_error::ConfigError;

#[derive(Clone, PartialEq, Eq)]
pub struct HttpConfig {
    pub(crate) connect_timeout: Duration,
    pub(crate) headers_timeout: Duration,
    pub(crate) idle_timeout: Duration,
    pub(crate) total_timeout: Option<Duration>,
    pub(crate) user_agent: Option<String>,
    pub(crate) extra_root_certificates: Vec<String>,
}

impl HttpConfig {
    pub fn new(
        connect_timeout: Duration,
        headers_timeout: Duration,
        idle_timeout: Duration,
    ) -> Self {
        Self {
            connect_timeout,
            headers_timeout,
            idle_timeout,
            total_timeout: None,
            user_agent: None,
            extra_root_certificates: Vec::new(),
        }
    }

    pub fn with_total_timeout(mut self, total_timeout: Duration) -> Self {
        self.total_timeout = Some(total_timeout);
        self
    }

    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    pub fn with_extra_root_certificates(
        mut self,
        certificates: impl IntoIterator<Item = String>,
    ) -> Self {
        self.extra_root_certificates = certificates.into_iter().collect();
        self
    }

    pub(crate) fn validate(&self) -> Result<HeaderValue, ConfigError> {
        let timeouts = [
            ("connect_timeout", Some(self.connect_timeout)),
            ("headers_timeout", Some(self.headers_timeout)),
            ("idle_timeout", Some(self.idle_timeout)),
            ("total_timeout", self.total_timeout),
        ];
        for (name, value) in timeouts {
            if value.is_some_and(|d| d.is_zero()) {
                return Err(ConfigError::config(format!("{name} cannot be zero")));
            }
        }
        self.resolved_user_agent()
    }

    fn resolved_user_agent(&self) -> Result<HeaderValue, ConfigError> {
        let default = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
        match &self.user_agent {
            None => Ok(HeaderValue::from_static(default)),
            Some(text) if text.is_empty() => Err(ConfigError::config("user_agent cannot be empty")),
            Some(text) => HeaderValue::from_str(text)
                .map_err(|_| ConfigError::config("user_agent is not a valid header value")),
        }
    }
}

impl std::fmt::Debug for HttpConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpConfig")
            .field("connect_timeout", &self.connect_timeout)
            .field("headers_timeout", &self.headers_timeout)
            .field("idle_timeout", &self.idle_timeout)
            .field("total_timeout", &self.total_timeout)
            .field("user_agent", &self.user_agent)
            .field(
                "extra_root_certificates",
                &self.extra_root_certificates.len(),
            )
            .finish()
    }
}
