use std::sync::Arc;

use br_llm_clients_common_http::Headers;
use br_llm_clients_core::{ClientError, ClientRequest, OutputMode};

use crate::dialect::Dialect;
use crate::dialect_hooks::DialectHooks;
use crate::encoded_request::EncodedRequest;
use crate::error_info::ErrorInfo;
use crate::payload_decoder::PayloadDecoder;

pub struct Layered<H> {
    inner: Arc<dyn Dialect>,
    hooks: H,
}

impl<H: DialectHooks> Layered<H> {
    pub fn new(inner: Arc<dyn Dialect>, hooks: H) -> Self {
        Self { inner, hooks }
    }
}

impl<H: DialectHooks> Dialect for Layered<H> {
    fn encode(&self, request: &ClientRequest) -> Result<EncodedRequest, ClientError> {
        self.hooks.check(request)?;
        let encoded = self.inner.encode(request)?;
        self.hooks.after_encode(encoded)
    }

    fn decoder(&self, output: &OutputMode) -> Box<dyn PayloadDecoder> {
        self.hooks.wrap_decoder(self.inner.decoder(output))
    }

    fn error_code(&self, body: &str) -> Option<String> {
        self.hooks.error_code(body, self.inner.error_code(body))
    }

    fn refine_error(&self, info: &ErrorInfo) -> Option<ClientError> {
        self.hooks
            .refine_error(info)
            .or_else(|| self.inner.refine_error(info))
    }

    fn request_id(&self, headers: &Headers) -> Option<String> {
        self.hooks
            .request_id(headers, self.inner.request_id(headers))
    }

    fn answered_model(&self, headers: &Headers) -> Option<String> {
        self.hooks
            .answered_model(headers, self.inner.answered_model(headers))
    }
}
