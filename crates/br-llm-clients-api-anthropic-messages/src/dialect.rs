use br_llm_clients_common_call::{Dialect, EncodedRequest, ErrorInfo, Headers, PayloadDecoder};
use br_llm_clients_core::{
    ClientError, ClientRequest, ClientSettings, OutputMode, RenderedHistory,
};

use crate::decode::Decoder;
use crate::encode::{self, EncodeSettings};
use crate::errors;
use crate::options::MessagesOptions;

pub struct AnthropicMessages {
    settings: EncodeSettings,
}

impl AnthropicMessages {
    pub fn new(settings: &ClientSettings) -> Self {
        Self::with_options(settings, MessagesOptions::new())
    }

    pub fn with_options(settings: &ClientSettings, options: MessagesOptions) -> Self {
        Self {
            settings: EncodeSettings::from_settings(settings, options),
        }
    }
}

impl Dialect for AnthropicMessages {
    fn encode(
        &self,
        request: &ClientRequest,
        history: &RenderedHistory,
    ) -> Result<EncodedRequest, ClientError> {
        encode::encode(&self.settings, request, history)
    }

    fn decoder(&self, output: &OutputMode) -> Box<dyn PayloadDecoder> {
        Box::new(Decoder::new(matches!(output, OutputMode::Structured(_))))
    }

    fn error_code(&self, body: &str) -> Option<String> {
        errors::error_code(body)
    }

    fn refine_error(&self, info: &ErrorInfo) -> Option<ClientError> {
        errors::refine_error(self.settings.reasoning(), info)
    }

    fn request_id(&self, headers: &Headers) -> Option<String> {
        errors::request_id(headers)
    }
}
