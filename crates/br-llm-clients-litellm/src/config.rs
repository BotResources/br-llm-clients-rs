use std::fmt;
use std::sync::Arc;

use br_llm_clients_common_http::{Endpoint, Secret};
use br_llm_clients_core::{Capabilities, IdSource};

use crate::api_family::ApiFamily;
use crate::key_placement::KeyPlacement;

#[derive(Clone)]
pub struct LiteLlmConfig {
    pub(crate) base: Endpoint,
    pub(crate) key: Secret,
    pub(crate) family: ApiFamily,
    pub(crate) capabilities: Capabilities,
    pub(crate) key_placement: KeyPlacement,
    pub(crate) extra_headers: Vec<(String, String)>,
    pub(crate) id_source: Option<Arc<dyn IdSource>>,
}

impl LiteLlmConfig {
    pub fn new(base: Endpoint, key: Secret, family: ApiFamily, capabilities: Capabilities) -> Self {
        Self {
            base,
            key,
            family,
            capabilities,
            key_placement: KeyPlacement::default(),
            extra_headers: Vec::new(),
            id_source: None,
        }
    }

    pub fn with_key_placement(mut self, key_placement: KeyPlacement) -> Self {
        self.key_placement = key_placement;
        self
    }

    pub fn with_extra_headers(mut self, extra_headers: Vec<(String, String)>) -> Self {
        self.extra_headers = extra_headers;
        self
    }

    pub fn with_id_source(mut self, id_source: Arc<dyn IdSource>) -> Self {
        self.id_source = Some(id_source);
        self
    }
}

impl fmt::Debug for LiteLlmConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let id_source = if self.id_source.is_some() {
            "set"
        } else {
            "default"
        };
        f.debug_struct("LiteLlmConfig")
            .field("base", &format_args!("{}", self.base))
            .field("family", &self.family.name())
            .field("key_placement", &self.key_placement.kind())
            .field("extra_headers", &self.extra_headers.len())
            .field("id_source", &id_source)
            .finish_non_exhaustive()
    }
}
