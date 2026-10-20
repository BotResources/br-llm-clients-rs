use br_llm_clients_common_call::{Endpoint, Secret};

#[derive(Debug, Clone)]
pub struct AnthropicConfig {
    pub(crate) api_key: Secret,
    pub(crate) base: Option<Endpoint>,
    pub(crate) workspace_id: Option<String>,
}

impl AnthropicConfig {
    pub fn new(api_key: Secret) -> Self {
        Self {
            api_key,
            base: None,
            workspace_id: None,
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
}
