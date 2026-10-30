use std::fmt;
use std::sync::Arc;

use br_llm_clients_api_openai_responses::{Map, PromptCacheMode, ServiceTier, Value};
use br_llm_clients_common_call::{Endpoint, Secret};
use br_llm_clients_core::IdSource;

use crate::profile::ModelProfile;

#[derive(Clone)]
pub struct OpenAiConfig {
    pub(crate) key: Secret,
    pub(crate) profile: ModelProfile,
    pub(crate) base: Option<Endpoint>,
    pub(crate) organization: Option<String>,
    pub(crate) project: Option<String>,
    pub(crate) prompt_cache_key: Option<String>,
    pub(crate) service_tier: Option<ServiceTier>,
    pub(crate) cache_mode: Option<PromptCacheMode>,
    pub(crate) extra_body: Map<String, Value>,
    pub(crate) id_source: Option<Arc<dyn IdSource>>,
}

impl OpenAiConfig {
    pub fn new(key: Secret, profile: ModelProfile) -> Self {
        Self {
            key,
            profile,
            base: None,
            organization: None,
            project: None,
            prompt_cache_key: None,
            service_tier: None,
            cache_mode: None,
            extra_body: Map::new(),
            id_source: None,
        }
    }

    pub fn with_base(mut self, base: Endpoint) -> Self {
        self.base = Some(base);
        self
    }

    pub fn with_organization(mut self, organization: impl Into<String>) -> Self {
        self.organization = Some(organization.into());
        self
    }

    pub fn with_project(mut self, project: impl Into<String>) -> Self {
        self.project = Some(project.into());
        self
    }

    pub fn with_prompt_cache_key(mut self, prompt_cache_key: impl Into<String>) -> Self {
        self.prompt_cache_key = Some(prompt_cache_key.into());
        self
    }

    pub fn with_service_tier(mut self, service_tier: ServiceTier) -> Self {
        self.service_tier = Some(service_tier);
        self
    }

    pub fn with_cache_mode(mut self, cache_mode: PromptCacheMode) -> Self {
        self.cache_mode = Some(cache_mode);
        self
    }

    pub fn with_extra_body(mut self, extra_body: Map<String, Value>) -> Self {
        self.extra_body = extra_body;
        self
    }

    pub fn with_id_source(mut self, id_source: Arc<dyn IdSource>) -> Self {
        self.id_source = Some(id_source);
        self
    }
}

impl fmt::Debug for OpenAiConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let members: Vec<&String> = self.extra_body.keys().collect();
        f.debug_struct("OpenAiConfig")
            .field("profile", &self.profile)
            .field("base", &self.base)
            .field("organization_set", &self.organization.is_some())
            .field("project_set", &self.project.is_some())
            .field("prompt_cache_key_set", &self.prompt_cache_key.is_some())
            .field("service_tier", &self.service_tier)
            .field("cache_mode", &self.cache_mode)
            .field("extra_body_members", &members)
            .field("id_source_set", &self.id_source.is_some())
            .finish_non_exhaustive()
    }
}
