use br_llm_clients_common_call::{DialectHooks, ErrorInfo, Headers};
use br_llm_clients_core::{Capabilities, ClientError, ClientRequest, ClientSettings};

use crate::api::Route;
use crate::refine::refine;

const REQUEST_ID_HEADER: &str = "apim-request-id";

pub(crate) struct AzureOpenAiHooks {
    capabilities: Option<Capabilities>,
    settings: ClientSettings,
    route: Route,
}

impl AzureOpenAiHooks {
    pub(crate) fn new(
        capabilities: Option<Capabilities>,
        settings: ClientSettings,
        route: Route,
    ) -> Self {
        Self {
            capabilities,
            settings,
            route,
        }
    }
}

impl DialectHooks for AzureOpenAiHooks {
    fn check(&self, request: &ClientRequest) -> Result<(), ClientError> {
        match &self.capabilities {
            Some(capabilities) => capabilities.check(&self.settings, request),
            None => Ok(()),
        }
    }

    fn refine_error(&self, info: &ErrorInfo) -> Option<ClientError> {
        refine(self.route, info)
    }

    fn request_id(&self, headers: &Headers, inner: Option<String>) -> Option<String> {
        headers
            .get_text(REQUEST_ID_HEADER)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .or(inner)
    }
}
