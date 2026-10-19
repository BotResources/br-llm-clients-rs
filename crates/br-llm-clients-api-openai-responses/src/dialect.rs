use br_llm_clients_common_call::{Dialect, EncodedRequest, ErrorInfo, Headers, PayloadDecoder};
use br_llm_clients_core::{ClientError, ClientRequest, ClientSettings, OutputMode, ReasoningLevel};

use crate::decode::ResponsesDecoder;
use crate::encode::{Fixed, encode};
use crate::error_code::error_code;
use crate::options::Options;
use crate::refine::refine_error;
use crate::request_id::request_id;

#[derive(Debug, Clone)]
pub struct OpenAiResponses {
    model: String,
    reasoning: ReasoningLevel,
    max_output_tokens: u32,
    options: Options,
}

impl OpenAiResponses {
    pub fn new(settings: &ClientSettings, options: Options) -> Self {
        Self {
            model: settings.model().as_str().to_owned(),
            reasoning: settings.reasoning(),
            max_output_tokens: settings.max_output_tokens().get(),
            options,
        }
    }
}

impl Dialect for OpenAiResponses {
    fn encode(&self, request: &ClientRequest) -> Result<EncodedRequest, ClientError> {
        encode(
            &Fixed {
                model: &self.model,
                reasoning: self.reasoning,
                max_output_tokens: self.max_output_tokens,
                options: &self.options,
            },
            request,
        )
    }

    fn decoder(&self, output: &OutputMode) -> Box<dyn PayloadDecoder> {
        Box::new(ResponsesDecoder::new(matches!(
            output,
            OutputMode::Structured(_)
        )))
    }

    fn error_code(&self, body: &str) -> Option<String> {
        error_code(body)
    }

    fn refine_error(&self, info: &ErrorInfo) -> Option<ClientError> {
        refine_error(info)
    }

    fn request_id(&self, headers: &Headers) -> Option<String> {
        request_id(headers)
    }
}
