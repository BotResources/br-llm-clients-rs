use br_llm_clients_common_call::{Dialect, EncodedRequest, ErrorInfo, Headers, PayloadDecoder};
use br_llm_clients_core::{ClientError, ClientRequest, ClientSettings, OutputMode};

use crate::decoder::ChatCompletionsDecoder;
use crate::encode::encode;
use crate::error_code::{error_code, refine_error};
use crate::options::ChatCompletionsOptions;
use crate::options_error::OptionsError;
use crate::request_id::request_id;

#[derive(Debug, Clone)]
pub struct ChatCompletionsDialect {
    settings: ClientSettings,
    options: ChatCompletionsOptions,
}

impl ChatCompletionsDialect {
    pub fn new(
        settings: ClientSettings,
        options: ChatCompletionsOptions,
    ) -> Result<Self, OptionsError> {
        options.check()?;
        Ok(Self { settings, options })
    }
}

impl Dialect for ChatCompletionsDialect {
    fn encode(&self, request: &ClientRequest) -> Result<EncodedRequest, ClientError> {
        encode(&self.settings, &self.options, request)
    }

    fn decoder(&self, output: &OutputMode) -> Box<dyn PayloadDecoder> {
        Box::new(ChatCompletionsDecoder::new(output, &self.options))
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
