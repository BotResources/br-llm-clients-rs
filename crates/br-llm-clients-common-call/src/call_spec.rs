use std::sync::Arc;

use br_llm_clients_common_http::{Authorizer, Endpoint, HttpClient, Secret};
use br_llm_messages::ModelId;

use crate::dialect::Dialect;
use crate::text_clean::Secrets;

pub struct CallSpec {
    pub(crate) dialect: Arc<dyn Dialect>,
    pub(crate) endpoint: Endpoint,
    pub(crate) authorizer: Arc<dyn Authorizer>,
    pub(crate) http: HttpClient,
    pub(crate) extra_headers: Vec<(String, String)>,
    pub(crate) secrets: Secrets,
    pub(crate) model: ModelId,
}

impl CallSpec {
    pub fn new(
        dialect: Arc<dyn Dialect>,
        endpoint: Endpoint,
        authorizer: Arc<dyn Authorizer>,
        http: HttpClient,
        model: ModelId,
    ) -> Self {
        Self {
            dialect,
            endpoint,
            authorizer,
            http,
            extra_headers: Vec::new(),
            secrets: Secrets::default(),
            model,
        }
    }

    pub fn with_extra_headers(mut self, headers: Vec<(String, String)>) -> Self {
        self.extra_headers = headers;
        self
    }

    pub fn with_secrets(mut self, secrets: Vec<Secret>) -> Self {
        self.secrets = Secrets::new(secrets);
        self
    }
}
