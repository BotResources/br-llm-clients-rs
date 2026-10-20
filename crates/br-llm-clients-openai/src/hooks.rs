use br_llm_clients_api_openai_responses::{
    OpenAiResponses, Options, ReasoningParameter, ReasoningSummary,
};
use br_llm_clients_common_call::{
    Dialect, EncodedRequest, ErrorInfo, Headers, PayloadDecoder, Secret,
};
use br_llm_clients_core::{Capabilities, ClientError, ClientRequest, ClientSettings, OutputMode};

use crate::profile::ModelProfile;
use crate::refine::refine;

pub(crate) struct OpenAiHooks {
    dialect: OpenAiResponses,
    capabilities: Capabilities,
    settings: ClientSettings,
    key: Secret,
}

impl OpenAiHooks {
    pub(crate) fn new(settings: &ClientSettings, profile: &ModelProfile, key: &Secret) -> Self {
        let reasoning_parameter = profile.reasoning_parameter();
        let reasoning_summary = if matches!(reasoning_parameter, ReasoningParameter::Omitted) {
            ReasoningSummary::None
        } else {
            profile.reasoning_summary()
        };
        let mut options = Options::default();
        options.reasoning_parameter = reasoning_parameter;
        options.reasoning_summary = reasoning_summary;
        Self {
            dialect: OpenAiResponses::new(settings, options),
            capabilities: profile.capabilities().clone(),
            settings: settings.clone(),
            key: key.clone(),
        }
    }
}

impl Dialect for OpenAiHooks {
    fn encode(&self, request: &ClientRequest) -> Result<EncodedRequest, ClientError> {
        self.capabilities.check(&self.settings, request)?;
        self.dialect.encode(request)
    }

    fn decoder(&self, output: &OutputMode) -> Box<dyn PayloadDecoder> {
        self.dialect.decoder(output)
    }

    fn error_code(&self, body: &str) -> Option<String> {
        self.dialect.error_code(body)
    }

    fn refine_error(&self, info: &ErrorInfo) -> Option<ClientError> {
        refine(info).or_else(|| self.dialect.refine_error(info))
    }

    fn request_id(&self, headers: &Headers) -> Option<String> {
        self.dialect
            .request_id(headers)
            .filter(|value| !value.contains(self.key.expose()))
    }
}
