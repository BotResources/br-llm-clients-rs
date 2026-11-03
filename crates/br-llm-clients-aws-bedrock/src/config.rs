use std::fmt;
use std::sync::Arc;

use br_llm_clients_common_call::Endpoint;
use br_llm_clients_core::IdSource;

use crate::credential::BedrockCredential;

#[derive(Clone)]
pub struct BedrockConfig {
    pub(crate) region: String,
    pub(crate) credential: BedrockCredential,
    pub(crate) base: Option<Endpoint>,
    pub(crate) id_source: Option<Arc<dyn IdSource>>,
}

impl BedrockConfig {
    pub fn new(region: impl Into<String>, credential: BedrockCredential) -> Self {
        Self {
            region: region.into(),
            credential,
            base: None,
            id_source: None,
        }
    }

    pub fn with_base(mut self, base: Endpoint) -> Self {
        self.base = Some(base);
        self
    }

    pub fn with_id_source(mut self, id_source: Arc<dyn IdSource>) -> Self {
        self.id_source = Some(id_source);
        self
    }
}

impl fmt::Debug for BedrockConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BedrockConfig")
            .field("region", &self.region)
            .field("credential", &self.credential)
            .field("base", &self.base)
            .field("id_source_set", &self.id_source.is_some())
            .finish()
    }
}
