use std::fmt;
use std::sync::Arc;

use br_llm_clients_common_call::{Endpoint, Secret};
use br_llm_clients_core::{Capabilities, IdSource};

use crate::api::AzureOpenAiApi;
use crate::version::AzureOpenAiVersion;

#[derive(Clone)]
pub struct AzureOpenAiConfig {
    pub(crate) endpoint: Endpoint,
    pub(crate) key: Secret,
    pub(crate) api: AzureOpenAiApi,
    pub(crate) version: AzureOpenAiVersion,
    pub(crate) capabilities: Option<Capabilities>,
    pub(crate) id_source: Option<Arc<dyn IdSource>>,
}

impl AzureOpenAiConfig {
    pub fn new(
        endpoint: Endpoint,
        key: Secret,
        api: AzureOpenAiApi,
        version: AzureOpenAiVersion,
    ) -> Self {
        Self {
            endpoint,
            key,
            api,
            version,
            capabilities: None,
            id_source: None,
        }
    }

    pub fn with_capabilities(mut self, capabilities: Capabilities) -> Self {
        self.capabilities = Some(capabilities);
        self
    }

    pub fn with_id_source(mut self, id_source: Arc<dyn IdSource>) -> Self {
        self.id_source = Some(id_source);
        self
    }
}

impl fmt::Debug for AzureOpenAiConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AzureOpenAiConfig")
            .field("endpoint", &self.endpoint)
            .field("api", &self.api)
            .field("version", &self.version)
            .field("capabilities", &self.capabilities)
            .field("id_source_set", &self.id_source.is_some())
            .finish_non_exhaustive()
    }
}
