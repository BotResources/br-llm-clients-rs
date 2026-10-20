use std::fmt;
use std::sync::Arc;

use br_llm_clients_common_call::{CallSpec, run};
use br_llm_clients_common_http::{Endpoint, HttpClient};
use br_llm_clients_core::{
    BoxFuture, CallOutcome, Client, ClientRequest, ClientSettings, StreamSink,
};

use crate::build_error::LiteLlmBuildError;
use crate::check_key::check_key;
use crate::config::LiteLlmConfig;
use crate::extra_headers::check_extra_headers;
use crate::hooks::LiteLlmHooks;
use crate::key_header::key_authorizer;
use crate::route::{route_dialect, route_endpoint};
use crate::secrets::guard_values;

#[derive(Clone)]
pub struct LiteLlmClient {
    settings: ClientSettings,
    spec: Arc<CallSpec>,
    endpoint: Endpoint,
    family: &'static str,
    key_placement: &'static str,
    extra_headers: usize,
}

impl LiteLlmClient {
    pub fn new(
        settings: ClientSettings,
        config: LiteLlmConfig,
        http: HttpClient,
    ) -> Result<Self, LiteLlmBuildError> {
        check_key(&config.key)?;
        let authorizer = key_authorizer(&config.key_placement, &config.key)?;
        check_extra_headers(&config.extra_headers, &config.base)?;
        let endpoint = route_endpoint(&config.base, &config.family)?;
        let route = route_dialect(&settings, &config.family)?;
        let guards = guard_values(&config.key);
        let hooks = LiteLlmHooks::new(
            settings.clone(),
            config.capabilities.clone(),
            route,
            guards.clone(),
        );
        let spec = CallSpec::new(
            Arc::new(hooks),
            endpoint.clone(),
            authorizer,
            http,
            settings.model().clone(),
        )
        .with_extra_headers(config.extra_headers.clone())
        .with_secrets(guards);
        Ok(Self {
            settings,
            spec: Arc::new(spec),
            endpoint,
            family: config.family.name(),
            key_placement: config.key_placement.kind(),
            extra_headers: config.extra_headers.len(),
        })
    }
}

impl Client for LiteLlmClient {
    fn call<'a>(
        &'a self,
        request: &'a ClientRequest,
        sink: &'a mut dyn StreamSink,
    ) -> BoxFuture<'a, CallOutcome> {
        Box::pin(async move { run(&self.spec, request, sink).await })
    }

    fn settings(&self) -> &ClientSettings {
        &self.settings
    }
}

impl fmt::Debug for LiteLlmClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LiteLlmClient")
            .field("endpoint", &format_args!("{}", self.endpoint))
            .field("family", &self.family)
            .field("key_placement", &self.key_placement)
            .field("extra_headers", &self.extra_headers)
            .finish_non_exhaustive()
    }
}
