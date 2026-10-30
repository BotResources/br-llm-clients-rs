use br_llm_clients_common_call::DialectHooks;
use br_llm_clients_core::{Capabilities, ClientError, ClientRequest, ClientSettings};

pub(crate) struct AnthropicHooks {
    settings: ClientSettings,
    capabilities: Capabilities,
}

impl AnthropicHooks {
    pub(crate) fn new(settings: ClientSettings, capabilities: Capabilities) -> Self {
        Self {
            settings,
            capabilities,
        }
    }
}

impl DialectHooks for AnthropicHooks {
    fn check(&self, request: &ClientRequest) -> Result<(), ClientError> {
        self.capabilities.check(&self.settings, request)
    }
}
