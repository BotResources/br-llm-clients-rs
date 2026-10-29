use std::fmt;
use std::sync::Arc;

use br_llm_clients_common_call::{CallSpec, Layered, run};
use br_llm_clients_common_http::{Endpoint, HttpClient};
use br_llm_clients_core::{
    BoxFuture, CallOutcome, Client, ClientRequest, ClientSettings, DefaultIdSource, StreamSink,
};

use crate::build_error::LiteLlmBuildError;
use crate::check_key::check_key;
use crate::config::LiteLlmConfig;
use crate::extra_headers::check_extra_headers;
use crate::hooks::LiteLlmHooks;
use crate::key_header::key_authorizer;
use crate::route::{route_dialect, route_endpoint};
use crate::secrets::key_secrets;

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
        let LiteLlmConfig {
            base,
            key,
            family,
            capabilities,
            key_placement,
            extra_headers,
            id_source,
        } = config;
        check_key(&key)?;
        let authorizer = key_authorizer(&key_placement, &key)?;
        check_extra_headers(&extra_headers, &base)?;
        let endpoint = route_endpoint(&base, &family)?;
        let route = route_dialect(&settings, &family)?;
        let hooks = LiteLlmHooks::new(settings.clone(), capabilities, Arc::clone(&route));
        let extra_header_count = extra_headers.len();
        let spec = CallSpec::new(
            Arc::new(Layered::new(route, hooks)),
            endpoint.clone(),
            authorizer,
            http,
        )
        .with_extra_headers(extra_headers)
        .with_secrets(key_secrets(&key))
        .with_id_source(id_source.unwrap_or_else(|| Arc::new(DefaultIdSource)));
        Ok(Self {
            settings,
            spec: Arc::new(spec),
            endpoint,
            family: family.name(),
            key_placement: key_placement.kind(),
            extra_headers: extra_header_count,
        })
    }
}

impl Client for LiteLlmClient {
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
