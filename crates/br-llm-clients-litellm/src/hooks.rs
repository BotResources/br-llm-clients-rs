use br_llm_clients_common_call::{Dialect, EncodedRequest, ErrorInfo, Headers, PayloadDecoder};
use br_llm_clients_common_http::Secret;
use br_llm_clients_core::{Capabilities, ClientError, ClientRequest, ClientSettings, OutputMode};

use crate::error_object;
use crate::refine::from_status;

const REQUEST_ID_HEADER: &str = "x-litellm-call-id";

pub(crate) struct LiteLlmHooks {
    settings: ClientSettings,
    capabilities: Capabilities,
    route: Box<dyn Dialect>,
    guards: Vec<Secret>,
}

impl LiteLlmHooks {
    pub(crate) fn new(
        settings: ClientSettings,
        capabilities: Capabilities,
        route: Box<dyn Dialect>,
        guards: Vec<Secret>,
    ) -> Self {
        Self {
            settings,
            capabilities,
            route,
            guards,
        }
    }
}

impl Dialect for LiteLlmHooks {
    fn encode(&self, request: &ClientRequest) -> Result<EncodedRequest, ClientError> {
        self.capabilities.check(&self.settings, request)?;
        self.route.encode(request)
    }

    fn decoder(&self, output: &OutputMode) -> Box<dyn PayloadDecoder> {
        self.route.decoder(output)
    }

    fn error_code(&self, body: &str) -> Option<String> {
        error_object::read(body)?.kind
    }

    fn refine_error(&self, info: &ErrorInfo) -> Option<ClientError> {
        if let Some(refined) = self.route.refine_error(info) {
            return Some(refined);
        }
        let object = error_object::read(info.text())?;
        from_status(info, &object)
    }

    fn request_id(&self, headers: &Headers) -> Option<String> {
        headers
            .get_text(REQUEST_ID_HEADER)
            .filter(|value| !value.is_empty())
            .filter(|value| {
                !self
                    .guards
                    .iter()
                    .any(|guard| value.contains(guard.expose()))
            })
            .map(str::to_owned)
    }
}
