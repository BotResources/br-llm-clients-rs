use std::fmt;
use std::sync::Arc;

use br_llm_clients_api_anthropic_messages::AnthropicMessages;
use br_llm_clients_common_call::{CallSpec, Layered, run};
use br_llm_clients_common_http::{Endpoint, HttpClient};
use br_llm_clients_core::{
    BoxFuture, CallOutcome, Client, ClientRequest, ClientSettings, StreamSink,
};

use crate::build_error::AnthropicBuildError;
use crate::capabilities::capabilities_of;
use crate::config::AnthropicConfig;
use crate::credential::{authorizer, check_key, check_workspace_id};
use crate::hooks::AnthropicHooks;

const DEFAULT_BASE: &str = "https://api.anthropic.com/";
const WORKSPACE_HEADER: &str = "anthropic-workspace-id";
const ROUTE: [&str; 2] = ["v1", "messages"];

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
            key_header,
            id_source,
        } = config;
        check_key(&api_key)?;
        check_workspace_id(workspace_id.as_deref())?;
        let endpoint = call_endpoint(base)?;
        let authorizer = authorizer(key_header, &api_key)?;
        let hooks =
            AnthropicHooks::new(settings.clone(), capabilities_of(settings.model().as_str()));
        let dialect = Layered::new(Arc::new(AnthropicMessages::new(&settings)), hooks);
        let extra_headers = workspace_id
            .map(|id| vec![(WORKSPACE_HEADER.to_owned(), id)])
            .unwrap_or_default();
        let mut spec = CallSpec::new(Arc::new(dialect), endpoint.clone(), authorizer, http)
            .with_extra_headers(extra_headers)
            .with_secrets(vec![api_key]);
        if let Some(id_source) = id_source {
            spec = spec.with_id_source(id_source);
        }
        Ok(Self {
            spec: Arc::new(spec),
            settings,
            endpoint,
        })
    }
}

fn call_endpoint(base: Option<Endpoint>) -> Result<Endpoint, AnthropicBuildError> {
    let mut endpoint = match base {
        Some(base) => base,
        None => Endpoint::https(DEFAULT_BASE)
            .map_err(|_| AnthropicBuildError::endpoint("the default base is not valid"))?,
    };
    for segment in ROUTE {
        endpoint = endpoint
            .join_segment(segment)
            .map_err(|_| AnthropicBuildError::endpoint("the endpoint of the call is too long"))?;
    }
    Ok(endpoint)
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
            .finish_non_exhaustive()
    }
}
