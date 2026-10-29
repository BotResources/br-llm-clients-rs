use std::sync::Arc;

use br_llm_clients_common_call::{Dialect, DialectHooks, ErrorInfo, Headers, PayloadDecoder};
use br_llm_clients_core::{Capabilities, ClientError, ClientRequest, ClientSettings};

use crate::error_object;
use crate::refine::refine;
use crate::stream_chunks::StreamChunks;

const REQUEST_ID_HEADER: &str = "x-generation-id";

pub(crate) struct OpenRouterHooks {
    settings: ClientSettings,
    capabilities: Capabilities,
    route: Arc<dyn Dialect>,
    chat: bool,
}

impl OpenRouterHooks {
    pub(crate) fn new(
        settings: ClientSettings,
        capabilities: Capabilities,
        route: Arc<dyn Dialect>,
        chat: bool,
    ) -> Self {
        Self {
            settings,
            capabilities,
            route,
            chat,
        }
    }
}

impl DialectHooks for OpenRouterHooks {
    fn check(&self, request: &ClientRequest) -> Result<(), ClientError> {
        self.capabilities.check(&self.settings, request)
    }

    fn wrap_decoder(&self, inner: Box<dyn PayloadDecoder>) -> Box<dyn PayloadDecoder> {
        if self.chat {
            Box::new(StreamChunks::new(inner))
        } else {
            inner
        }
    }

    fn error_code(&self, body: &str, inner: Option<String>) -> Option<String> {
        inner.or_else(|| {
            error_object::read(body).and_then(|object| object.error_type.or(object.code))
        })
    }

    fn refine_error(&self, info: &ErrorInfo) -> Option<ClientError> {
        refine(&*self.route, info)
    }

    fn request_id(&self, headers: &Headers, _inner: Option<String>) -> Option<String> {
        headers
            .get_text(REQUEST_ID_HEADER)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    }
}
