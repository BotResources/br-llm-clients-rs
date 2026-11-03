use br_llm_clients_common_call::{DialectHooks, ErrorInfo};
use br_llm_clients_core::{Capabilities, ClientError, ClientRequest, ClientSettings};

use crate::refine::refine;

pub(crate) struct MlxVlmHooks {
    settings: ClientSettings,
    capabilities: Capabilities,
}

impl MlxVlmHooks {
    pub(crate) fn new(settings: ClientSettings, capabilities: Capabilities) -> Self {
        Self {
            settings,
            capabilities,
        }
    }
}

impl DialectHooks for MlxVlmHooks {
    fn check(&self, request: &ClientRequest) -> Result<(), ClientError> {
        self.capabilities.check(&self.settings, request)
    }

    fn refine_error(&self, info: &ErrorInfo) -> Option<ClientError> {
        refine(info)
    }
}
