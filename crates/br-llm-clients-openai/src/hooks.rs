use br_llm_clients_common_call::{DialectHooks, ErrorInfo};
use br_llm_clients_core::{Capabilities, ClientError, ClientRequest, ClientSettings};

use crate::refine::refine;

pub(crate) struct OpenAiHooks {
    capabilities: Capabilities,
    settings: ClientSettings,
}

impl OpenAiHooks {
    pub(crate) fn new(capabilities: Capabilities, settings: ClientSettings) -> Self {
        Self {
            capabilities,
            settings,
        }
    }
}

impl DialectHooks for OpenAiHooks {
    fn check(&self, request: &ClientRequest) -> Result<(), ClientError> {
        self.capabilities.check(&self.settings, request)
    }

    fn refine_error(&self, info: &ErrorInfo) -> Option<ClientError> {
        refine(info)
    }
}
