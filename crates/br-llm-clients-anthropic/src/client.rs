use std::fmt;
use std::sync::Arc;

use br_llm_clients_common_call::{CallSpec, Endpoint, HttpClient, run};
use br_llm_clients_common_http::ApiKeyAuthorizer;
use br_llm_clients_core::{
    BoxFuture, CallOutcome, Client, ClientRequest, ClientSettings, StreamSink,
};

use crate::build_error::AnthropicBuildError;
use crate::capabilities::capabilities_of;
use crate::config::AnthropicConfig;
use crate::hooks::AnthropicHooks;
use crate::visible::is_visible_text;

const DEFAULT_BASE: &str = "https://api.anthropic.com/";
const KEY_HEADER: &str = "x-api-key";
const WORKSPACE_HEADER: &str = "anthropic-workspace-id";

#[derive(Clone)]
pub struct AnthropicClient {
    spec: Arc<CallSpec>,
    settings: ClientSettings,
    endpoint: Endpoint,
}

impl AnthropicClient {
    pub fn new(
        settings: ClientSettings,
        config: AnthropicConfig,
        http: HttpClient,
    ) -> Result<Self, AnthropicBuildError> {
        let AnthropicConfig {
            api_key,
            base,
            workspace_id,
        } = config;
        if !is_visible_text(api_key.expose()) {
            return Err(AnthropicBuildError::credential(
                "the key must hold only visible characters",
            ));
        }
        if let Some(id) = &workspace_id
            && !is_visible_text(id)
        {
            return Err(AnthropicBuildError::workspace_id(
                "the workspace id must be a non-empty text of visible characters",
            ));
        }
        let base = match base {
            Some(base) => base,
            None => Endpoint::https(DEFAULT_BASE)
                .map_err(|_| AnthropicBuildError::endpoint("the default base is not valid"))?,
        };
        let endpoint = base
            .join_segment("v1")
            .and_then(|versioned| versioned.join_segment("messages"))
            .map_err(|_| AnthropicBuildError::endpoint("the endpoint of the call is too long"))?;
        let authorizer = ApiKeyAuthorizer::new(KEY_HEADER, api_key.clone())
            .map_err(|_| AnthropicBuildError::credential("the key cannot be used as a header"))?;
        let capabilities = capabilities_of(settings.model().as_str());
        let hooks = AnthropicHooks::new(settings.clone(), capabilities, api_key.clone());
        let extra_headers = workspace_id
            .map(|id| vec![(WORKSPACE_HEADER.to_owned(), id)])
            .unwrap_or_default();
        let spec = CallSpec::new(
            Arc::new(hooks),
            endpoint.clone(),
            Arc::new(authorizer),
            http,
            settings.model().clone(),
        )
        .with_extra_headers(extra_headers)
        .with_secrets(vec![api_key]);
        Ok(Self {
            spec: Arc::new(spec),
            settings,
            endpoint,
        })
    }
}

impl Client for AnthropicClient {
    fn call<'a>(
        &'a self,
        request: &'a ClientRequest,
        sink: &'a mut dyn StreamSink,
    ) -> BoxFuture<'a, CallOutcome> {
        Box::pin(run(&self.spec, request, sink))
    }

    fn settings(&self) -> &ClientSettings {
        &self.settings
    }
}

impl fmt::Debug for AnthropicClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnthropicClient")
            .field("model", &self.settings.model().as_str())
            .field("endpoint", &format_args!("{}", self.endpoint))
            .finish()
    }
}
