use br_llm_clients_api_anthropic_messages::AnthropicMessages;
use br_llm_clients_common_call::{
    Dialect, EncodedRequest, ErrorInfo, Headers, PayloadDecoder, Secret,
};
use br_llm_clients_core::{Capabilities, ClientError, ClientRequest, ClientSettings, OutputMode};

pub(crate) struct AnthropicHooks {
    inner: AnthropicMessages,
    capabilities: Capabilities,
    settings: ClientSettings,
    key: Secret,
}

impl AnthropicHooks {
    pub(crate) fn new(settings: ClientSettings, capabilities: Capabilities, key: Secret) -> Self {
        Self {
            inner: AnthropicMessages::new(&settings),
            capabilities,
            settings,
            key,
        }
    }
}

impl Dialect for AnthropicHooks {
    fn encode(&self, request: &ClientRequest) -> Result<EncodedRequest, ClientError> {
        self.capabilities.check(&self.settings, request)?;
        self.inner.encode(request)
    }

    fn decoder(&self, output: &OutputMode) -> Box<dyn PayloadDecoder> {
        self.inner.decoder(output)
    }

    fn error_code(&self, body: &str) -> Option<String> {
        self.inner.error_code(body)
    }

    fn refine_error(&self, info: &ErrorInfo) -> Option<ClientError> {
        self.inner.refine_error(info)
    }

    fn request_id(&self, headers: &Headers) -> Option<String> {
        self.inner
            .request_id(headers)
            .filter(|id| !id.contains(self.key.expose()))
    }
}
