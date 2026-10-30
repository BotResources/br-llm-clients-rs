use std::fmt;
use std::sync::Arc;

use br_llm_clients_common_http::{Endpoint, Secret};
use br_llm_clients_core::IdSource;

#[derive(Clone, Copy)]
pub(crate) enum KeyHeader {
    ApiKey,
    Bearer,
}

impl KeyHeader {
    pub(crate) fn name(self) -> &'static str {
        match self {
            KeyHeader::ApiKey => "x-api-key",
            KeyHeader::Bearer => "authorization",
        }
    }
}

#[derive(Clone)]
pub struct AnthropicConfig {
    pub(crate) api_key: Secret,
    pub(crate) base: Option<Endpoint>,
    pub(crate) workspace_id: Option<String>,
    pub(crate) key_header: KeyHeader,
    pub(crate) id_source: Option<Arc<dyn IdSource>>,
}

impl AnthropicConfig {
    pub fn new(api_key: Secret) -> Self {
        Self {
            api_key,
            base: None,
            workspace_id: None,
            key_header: KeyHeader::ApiKey,
            id_source: None,
        }
    }

    pub fn with_base(mut self, base: Endpoint) -> Self {
        self.base = Some(base);
        self
    }

    pub fn with_workspace_id(mut self, workspace_id: impl Into<String>) -> Self {
        self.workspace_id = Some(workspace_id.into());
        self
    }

    pub fn with_bearer_auth(mut self) -> Self {
        self.key_header = KeyHeader::Bearer;
        self
    }

    pub fn with_id_source(mut self, id_source: Arc<dyn IdSource>) -> Self {
        self.id_source = Some(id_source);
        self
    }
}

impl fmt::Debug for AnthropicConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnthropicConfig")
            .field("base", &self.base)
            .field("workspace_id", &self.workspace_id)
            .field("key_header", &self.key_header.name())
            .field("id_source_set", &self.id_source.is_some())
            .finish_non_exhaustive()
    }
}
