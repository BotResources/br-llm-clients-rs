use std::sync::{Arc, OnceLock};

use br_llm_clients_common_call::{Dialect, DialectHooks, ErrorInfo, Headers};
use br_llm_clients_core::{Capabilities, ClientError, ClientRequest, ClientSettings};

use crate::error_object;
use crate::refine::refine;

const REQUEST_ID_HEADER: &str = "x-litellm-call-id";
const DEPLOYMENT_ID_HEADER: &str = "x-litellm-model-id";

pub(crate) type DeploymentSlot = Arc<OnceLock<String>>;

pub(crate) struct LiteLlmHooks {
    settings: ClientSettings,
    capabilities: Capabilities,
    route: Arc<dyn Dialect>,
    deployment: Option<DeploymentSlot>,
}

impl LiteLlmHooks {
    pub(crate) fn new(
        settings: ClientSettings,
        capabilities: Capabilities,
        route: Arc<dyn Dialect>,
    ) -> Self {
        Self {
            settings,
            capabilities,
            route,
            deployment: None,
        }
    }

    pub(crate) fn with_deployment_slot(mut self, slot: DeploymentSlot) -> Self {
        self.deployment = Some(slot);
        self
    }
}

fn header_text(headers: &Headers, name: &str) -> Option<String> {
    headers
        .get_text(name)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

impl DialectHooks for LiteLlmHooks {
    fn check(&self, request: &ClientRequest) -> Result<(), ClientError> {
        self.capabilities.check(&self.settings, request)
    }

    fn error_code(&self, body: &str, _inner: Option<String>) -> Option<String> {
        error_object::read(body)?.kind
    }

    fn refine_error(&self, info: &ErrorInfo) -> Option<ClientError> {
        refine(&*self.route, info)
    }

    fn request_id(&self, headers: &Headers, _inner: Option<String>) -> Option<String> {
        if let Some(slot) = &self.deployment
            && let Some(deployment) = header_text(headers, DEPLOYMENT_ID_HEADER)
        {
            let _ = slot.set(deployment);
        }
        header_text(headers, REQUEST_ID_HEADER)
    }
}
