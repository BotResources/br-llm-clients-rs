use std::fmt;
use std::sync::Arc;

use br_llm_clients_common_call::Endpoint;
use br_llm_clients_core::IdSource;

use crate::credential::VertexCredential;

#[derive(Clone)]
pub struct VertexConfig {
    pub(crate) project: String,
    pub(crate) location: String,
    pub(crate) credential: VertexCredential,
    pub(crate) base: Option<Endpoint>,
    pub(crate) id_source: Option<Arc<dyn IdSource>>,
}

impl VertexConfig {
    pub fn new(
        project: impl Into<String>,
        location: impl Into<String>,
        credential: VertexCredential,
    ) -> Self {
        Self {
            project: project.into(),
            location: location.into(),
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

impl fmt::Debug for VertexConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VertexConfig")
            .field("project", &self.project)
            .field("location", &self.location)
            .field("credential", &self.credential)
            .field("base", &self.base)
            .field("id_source_set", &self.id_source.is_some())
            .finish()
    }
}
