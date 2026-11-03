use br_llm_clients_common_call::{DialectHooks, Headers};
use br_llm_clients_core::{Capabilities, ClientError, ClientRequest, ClientSettings};

const AWS_REQUEST_ID_HEADER: &str = "x-amzn-requestid";

pub(crate) struct BedrockHooks {
    capabilities: Capabilities,
    settings: ClientSettings,
}

impl BedrockHooks {
    pub(crate) fn new(capabilities: Capabilities, settings: ClientSettings) -> Self {
        Self {
            capabilities,
            settings,
        }
    }
}

impl DialectHooks for BedrockHooks {
    fn check(&self, request: &ClientRequest) -> Result<(), ClientError> {
        self.capabilities.check(&self.settings, request)
    }

    fn request_id(&self, headers: &Headers, inner: Option<String>) -> Option<String> {
        inner.or_else(|| {
            headers
                .get_text(AWS_REQUEST_ID_HEADER)
                .filter(|id| !id.is_empty())
                .map(str::to_owned)
        })
    }
}
