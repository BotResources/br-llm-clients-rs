use std::fmt;
use std::sync::Arc;

use br_llm_clients_common_http::{Endpoint, Secret};
use br_llm_clients_core::{Capabilities, IdSource};

use crate::api_family::ApiFamily;

pub const DEFAULT_BASE: &str = "https://openrouter.ai/api/v1";

#[derive(Clone)]
pub struct OpenRouterConfig {
    pub(crate) key: Secret,
    pub(crate) family: ApiFamily,
    pub(crate) capabilities: Capabilities,
    pub(crate) base: Option<Endpoint>,
    pub(crate) app_referer: Option<String>,
    pub(crate) app_title: Option<String>,
    pub(crate) extra_headers: Vec<(String, String)>,
    pub(crate) id_source: Option<Arc<dyn IdSource>>,
}

impl OpenRouterConfig {
    pub fn new(key: Secret, family: ApiFamily, capabilities: Capabilities) -> Self {
        Self {
            key,
            family,
            capabilities,
            base: None,
            app_referer: None,
            app_title: None,
            extra_headers: Vec::new(),
            id_source: None,
        }
    }

    pub fn with_base(mut self, base: Endpoint) -> Self {
        self.base = Some(base);
        self
    }

    pub fn with_app_referer(mut self, referer: impl Into<String>) -> Self {
        self.app_referer = Some(referer.into());
        self
    }

    pub fn with_app_title(mut self, title: impl Into<String>) -> Self {
        self.app_title = Some(title.into());
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

impl fmt::Debug for OpenRouterConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let base = match &self.base {
            Some(base) => base.to_string(),
            None => DEFAULT_BASE.to_owned(),
        };
        let id_source = if self.id_source.is_some() {
            "set"
        } else {
            "default"
        };
        f.debug_struct("OpenRouterConfig")
            .field("base", &base)
            .field("family", &self.family.name())
            .field("app_referer", &self.app_referer)
            .field("app_title", &self.app_title)
            .field("extra_headers", &self.extra_headers.len())
            .field("id_source", &id_source)
            .finish_non_exhaustive()
    }
}
