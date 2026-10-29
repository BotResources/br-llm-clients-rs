use std::sync::Arc;

use br_llm_clients_common_http::{Authorizer, Endpoint, HttpClient, Secret};
use br_llm_clients_core::{DefaultIdSource, IdSource};

use crate::dialect::Dialect;
use crate::secret_log::SecretLog;
use crate::text_clean::Secrets;

pub struct CallSpec {
    pub(crate) dialect: Arc<dyn Dialect>,
    pub(crate) endpoint: Endpoint,
    pub(crate) authorizer: Arc<dyn Authorizer>,
    pub(crate) http: HttpClient,
    pub(crate) extra_headers: Vec<(String, String)>,
    pub(crate) secrets: Vec<Secret>,
    pub(crate) secret_log: SecretLog,
    pub(crate) ids: Arc<dyn IdSource>,
}

impl CallSpec {
    pub fn new(
        dialect: Arc<dyn Dialect>,
        endpoint: Endpoint,
        authorizer: Arc<dyn Authorizer>,
        http: HttpClient,
    ) -> Self {
        Self {
            dialect,
            endpoint,
            authorizer,
            http,
            extra_headers: Vec::new(),
            secrets: Vec::new(),
            secret_log: SecretLog::new(),
            ids: Arc::new(DefaultIdSource),
        }
    }

    pub fn with_extra_headers(mut self, headers: Vec<(String, String)>) -> Self {
        self.extra_headers = headers;
        self
    }

    pub fn with_secrets(mut self, secrets: Vec<Secret>) -> Self {
        self.secrets = secrets;
        self
    }

    pub fn with_secret_log(mut self, log: SecretLog) -> Self {
        self.secret_log = log;
        self
    }

    pub fn with_id_source(mut self, ids: Arc<dyn IdSource>) -> Self {
        self.ids = ids;
        self
    }

    pub(crate) fn secrets_with(&self, logged: Vec<Secret>) -> Secrets {
        let mut all = self.secrets.clone();
        all.extend(logged);
        Secrets::new(all)
    }
}
