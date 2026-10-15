use std::fmt;
use std::sync::Arc;

use http::HeaderValue;
use tokio::time::Instant;
use tokio_rustls::TlsConnector;

use crate::authorizer::Authorizer;
use crate::config::HttpConfig;
use crate::config_error::ConfigError;
use crate::deadline;
use crate::flow;
use crate::pool::Pool;
use crate::request::Request;
use crate::response::Response;
use crate::send_error::SendError;
use crate::tls;
use crate::tuning::Tuning;

pub(crate) struct Shared {
    pub(crate) config: HttpConfig,
    pub(crate) user_agent: HeaderValue,
    pub(crate) tls: TlsConnector,
    pub(crate) pool: Arc<Pool>,
    pub(crate) tuning: Tuning,
}

#[derive(Clone)]
pub struct HttpClient {
    shared: Arc<Shared>,
}

impl HttpClient {
    pub fn new(config: HttpConfig) -> Result<Self, ConfigError> {
        Self::build(config, Tuning::FIXED)
    }

    #[cfg(test)]
    pub(crate) fn with_tuning(config: HttpConfig, tuning: Tuning) -> Result<Self, ConfigError> {
        Self::build(config, tuning)
    }

    fn build(config: HttpConfig, tuning: Tuning) -> Result<Self, ConfigError> {
        let user_agent = config.validate()?;
        let tls = tls::build_connector(&config.extra_root_certificates)?;
        Ok(Self {
            shared: Arc::new(Shared {
                config,
                user_agent,
                tls,
                pool: Arc::new(Pool::new(tuning.idle_limit)),
                tuning,
            }),
        })
    }

    #[cfg(test)]
    pub(crate) fn tuning(&self) -> Tuning {
        self.shared.tuning
    }

    #[cfg(test)]
    pub(crate) fn shared(&self) -> &Arc<Shared> {
        &self.shared
    }

    #[cfg(test)]
    pub(crate) fn pool(&self) -> &Arc<Pool> {
        &self.shared.pool
    }

    #[cfg(test)]
    pub(crate) fn h2_connections(&self) -> usize {
        self.shared.pool.h2_connections()
    }

    pub async fn send(
        &self,
        request: Request,
        authorizer: &dyn Authorizer,
    ) -> Result<Response, SendError> {
        let total = self
            .shared
            .config
            .total_timeout
            .map(|limit| deadline::after(Instant::now(), limit));
        flow::run(&self.shared, request, authorizer, total).await
    }
}

impl fmt::Debug for HttpClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let config = &self.shared.config;
        f.debug_struct("HttpClient")
            .field("connect_timeout", &config.connect_timeout)
            .field("headers_timeout", &config.headers_timeout)
            .field("idle_timeout", &config.idle_timeout)
            .field("total_timeout", &config.total_timeout)
            .field(
                "extra_root_certificates",
                &config.extra_root_certificates.len(),
            )
            .finish()
    }
}
