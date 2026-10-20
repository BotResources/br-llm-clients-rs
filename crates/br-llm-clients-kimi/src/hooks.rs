use br_llm_clients_api_openai_chat_completions::ChatCompletionsDialect;
use br_llm_clients_common_call::{
    Dialect, EncodedRequest, ErrorInfo, Headers, PayloadDecoder, Secret,
};
use br_llm_clients_core::{Capabilities, ClientError, ClientRequest, ClientSettings, OutputMode};
use br_llm_messages::ModelId;

use crate::kimi_thinking::KimiThinking;
use crate::rewrite::rewrite;

pub(crate) struct KimiHooks {
    inner: ChatCompletionsDialect,
    settings: ClientSettings,
    thinking: Option<KimiThinking>,
    capabilities: Capabilities,
    key: Secret,
}

impl KimiHooks {
    pub(crate) fn new(
        inner: ChatCompletionsDialect,
        settings: ClientSettings,
        thinking: Option<KimiThinking>,
        capabilities: Capabilities,
        key: Secret,
    ) -> Self {
        Self {
            inner,
            settings,
            thinking,
            capabilities,
            key,
        }
    }
}

impl Dialect for KimiHooks {
    fn encode(&self, request: &ClientRequest) -> Result<EncodedRequest, ClientError> {
        let Some(kind) = &self.thinking else {
            return Err(unknown_model(self.settings.model()));
        };
        self.capabilities.check(&self.settings, request)?;
        let encoded = self.inner.encode(request)?;
        rewrite(&encoded, request, &self.settings, kind)
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

fn unknown_model(model: &ModelId) -> ClientError {
    ClientError::ProviderRejected {
        status: None,
        code: Some("kimi.unknown_model".to_owned()),
        body: format!("unknown model {model}: give a KimiThinking for it in the KimiConfig"),
    }
}
