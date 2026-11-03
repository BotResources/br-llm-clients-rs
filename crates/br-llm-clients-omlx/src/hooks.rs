use br_llm_clients_common_call::{DialectHooks, ErrorInfo, Headers};
use br_llm_clients_core::{Capabilities, ClientError, ClientRequest, ClientSettings};

use crate::refine::refine;

const REQUEST_ID_HEADER: &str = "x-omlx-request-id";

pub(crate) struct OmlxHooks {
    settings: ClientSettings,
    capabilities: Capabilities,
}

impl OmlxHooks {
    pub(crate) fn new(settings: ClientSettings, capabilities: Capabilities) -> Self {
        Self {
            settings,
            capabilities,
        }
    }
}

impl DialectHooks for OmlxHooks {
    fn check(&self, request: &ClientRequest) -> Result<(), ClientError> {
        self.capabilities.check(&self.settings, request)
    }

    fn refine_error(&self, info: &ErrorInfo) -> Option<ClientError> {
        refine(info)
    }

    fn request_id(&self, headers: &Headers, _inner: Option<String>) -> Option<String> {
        headers
            .get_text(REQUEST_ID_HEADER)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    }
}
