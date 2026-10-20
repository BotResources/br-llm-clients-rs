use br_llm_clients_common_http::Headers;
use br_llm_clients_core::{ClientError, ClientRequest, OutputMode};

use crate::encoded_request::EncodedRequest;
use crate::error_info::ErrorInfo;
use crate::payload_decoder::PayloadDecoder;

pub trait Dialect: Send + Sync {
    fn encode(&self, request: &ClientRequest) -> Result<EncodedRequest, ClientError>;

    fn decoder(&self, output: &OutputMode) -> Box<dyn PayloadDecoder>;

    fn error_code(&self, body: &str) -> Option<String>;

    fn refine_error(&self, info: &ErrorInfo) -> Option<ClientError>;

    fn request_id(&self, headers: &Headers) -> Option<String>;

    fn answered_model(&self, _headers: &Headers) -> Option<String> {
        None
    }
}
