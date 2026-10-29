use br_llm_clients_common_call::{DialectHooks, ErrorInfo};
use br_llm_clients_core::{Capabilities, ClientError, ClientRequest, ClientSettings};
use serde_json::Value;

const MAX_STATUS_BYTES: usize = 64;
const NOT_FOUND_STATUS: u16 = 404;
const NOT_FOUND_CODE: &str = "NOT_FOUND";

pub(crate) struct VertexHooks {
    capabilities: Capabilities,
    settings: ClientSettings,
}

impl VertexHooks {
    pub(crate) fn new(capabilities: Capabilities, settings: ClientSettings) -> Self {
        Self {
            capabilities,
            settings,
        }
    }
}

impl DialectHooks for VertexHooks {
    fn check(&self, request: &ClientRequest) -> Result<(), ClientError> {
        self.capabilities.check(&self.settings, request)
    }

    fn error_code(&self, body: &str, inner: Option<String>) -> Option<String> {
        inner.or_else(|| google_status(body))
    }

    fn refine_error(&self, info: &ErrorInfo) -> Option<ClientError> {
        (info.status() == Some(NOT_FOUND_STATUS) && info.code() == Some(NOT_FOUND_CODE))
            .then_some(ClientError::ModelNotFound)
    }
}

fn google_status(body: &str) -> Option<String> {
    let parsed = serde_json::from_str::<Value>(body).ok()?;
    let status = parsed.get("error")?.as_object()?.get("status")?.as_str()?;
    is_status(status).then(|| status.to_owned())
}

fn is_status(status: &str) -> bool {
    !status.is_empty()
        && status.len() <= MAX_STATUS_BYTES
        && status
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte == b'_')
}
