use br_llm_clients_common_http::Headers;
use br_llm_clients_core::{ClientError, ClientRequest};

use crate::encoded_request::EncodedRequest;
use crate::error_info::ErrorInfo;
use crate::payload_decoder::PayloadDecoder;

pub trait DialectHooks: Send + Sync {
    fn check(&self, _request: &ClientRequest) -> Result<(), ClientError> {
        Ok(())
    }

    fn after_encode(&self, encoded: EncodedRequest) -> Result<EncodedRequest, ClientError> {
        Ok(encoded)
    }

    fn wrap_decoder(&self, inner: Box<dyn PayloadDecoder>) -> Box<dyn PayloadDecoder> {
        inner
    }

    fn error_code(&self, _body: &str, inner: Option<String>) -> Option<String> {
        inner
    }

    fn refine_error(&self, _info: &ErrorInfo) -> Option<ClientError> {
        None
    }

    fn request_id(&self, _headers: &Headers, inner: Option<String>) -> Option<String> {
        inner
    }

    fn answered_model(&self, _headers: &Headers, inner: Option<String>) -> Option<String> {
        inner
    }
}
