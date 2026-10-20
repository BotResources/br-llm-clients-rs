use std::fmt;

use br_llm_clients_common_call::{Endpoint, Secret};

use crate::profile::ModelProfile;

#[derive(Clone)]
pub struct OpenAiConfig {
    pub(crate) key: Secret,
    pub(crate) profile: ModelProfile,
    pub(crate) base: Option<Endpoint>,
    pub(crate) organization: Option<String>,
    pub(crate) project: Option<String>,
}

impl OpenAiConfig {
    pub fn new(key: Secret, profile: ModelProfile) -> Self {
        Self {
            key,
            profile,
            base: None,
            organization: None,
            project: None,
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
}

impl fmt::Debug for OpenAiConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenAiConfig")
            .field("profile", &self.profile)
            .field("base", &self.base)
            .field("organization_set", &self.organization.is_some())
            .field("project_set", &self.project.is_some())
            .finish_non_exhaustive()
    }
}
