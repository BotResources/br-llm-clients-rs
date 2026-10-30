use std::sync::Arc;

use br_llm_clients_common_call::{Dialect, DialectHooks, ErrorInfo, Headers};
use br_llm_clients_core::{Capabilities, ClientError, ClientRequest, ClientSettings};

use crate::error_object;
use crate::refine::refine;

const REQUEST_ID_HEADER: &str = "x-litellm-call-id";

pub(crate) struct LiteLlmHooks {
    settings: ClientSettings,
    capabilities: Capabilities,
    route: Arc<dyn Dialect>,
}

impl LiteLlmHooks {
    pub(crate) fn new(
        settings: ClientSettings,
        capabilities: Capabilities,
        route: Arc<dyn Dialect>,
    ) -> Self {
        Self {
            settings,
            capabilities,
            route,
        }
    }
}

impl DialectHooks for LiteLlmHooks {
    fn check(&self, request: &ClientRequest) -> Result<(), ClientError> {
        self.capabilities.check(&self.settings, request)
    }

    fn error_code(&self, body: &str, _inner: Option<String>) -> Option<String> {
        error_object::read(body)?.kind
    }

    fn refine_error(&self, info: &ErrorInfo) -> Option<ClientError> {
        refine(&*self.route, info)
    }

    fn request_id(&self, headers: &Headers, _inner: Option<String>) -> Option<String> {
        headers
            .get_text(REQUEST_ID_HEADER)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    }
}
