use std::fmt;
use std::sync::{Arc, OnceLock};

use br_llm_clients_common_call::{Authorizer, CallSpec, Dialect, Layered, run};
use br_llm_clients_common_http::{Endpoint, HttpClient, Secret};
use br_llm_clients_core::{
    BoxFuture, CallOutcome, Capabilities, Client, ClientRequest, ClientSettings, DefaultIdSource,
    IdSource, StreamSink,
};

use crate::build_error::LiteLlmBuildError;
use crate::call_options::LiteLlmCallOptions;
use crate::call_outcome::LiteLlmCallOutcome;
use crate::check_key::check_key;
use crate::config::LiteLlmConfig;
use crate::extra_headers::check_extra_headers;
use crate::hooks::LiteLlmHooks;
use crate::key_header::key_authorizer;
use crate::route::{route_dialect, route_endpoint};
use crate::secrets::key_secrets;

const SESSION_ID_HEADER: &str = "x-litellm-session-id";

struct Parts {
    settings: ClientSettings,
    capabilities: Capabilities,
    route: Arc<dyn Dialect>,
    endpoint: Endpoint,
    authorizer: Arc<dyn Authorizer>,
    http: HttpClient,
    extra_headers: Vec<(String, String)>,
    secrets: Vec<Secret>,
    ids: Arc<dyn IdSource>,
}

impl Parts {
    fn spec(&self, hooks: LiteLlmHooks, extra_headers: Vec<(String, String)>) -> CallSpec {
        CallSpec::new(
            Arc::new(Layered::new(Arc::clone(&self.route), hooks)),
            self.endpoint.clone(),
            Arc::clone(&self.authorizer),
            self.http.clone(),
        )
        .with_extra_headers(extra_headers)
        .with_secrets(self.secrets.clone())
        .with_id_source(Arc::clone(&self.ids))
    }

    fn hooks(&self) -> LiteLlmHooks {
        LiteLlmHooks::new(
            self.settings.clone(),
            self.capabilities.clone(),
            Arc::clone(&self.route),
        )
    }

    fn extra_headers_for(&self, options: &LiteLlmCallOptions) -> Vec<(String, String)> {
        let Some(session_id) = options.session_id() else {
            return self.extra_headers.clone();
        };
        self.extra_headers
            .iter()
            .filter(|(name, _)| !name.eq_ignore_ascii_case(SESSION_ID_HEADER))
            .cloned()
            .chain([(SESSION_ID_HEADER.to_owned(), session_id.as_str().to_owned())])
            .collect()
    }

    fn deployment_id(&self, slot: &OnceLock<String>) -> Option<String> {
        slot.get()
            .filter(|value| !self.secrets.iter().any(|secret| secret.appears_in(value)))
            .cloned()
    }
}

#[derive(Clone)]
pub struct LiteLlmClient {
    parts: Arc<Parts>,
    spec: Arc<CallSpec>,
    family: &'static str,
    key_placement: &'static str,
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
        let parts = Parts {
            settings,
            capabilities,
            route,
            endpoint,
            authorizer,
            http,
            extra_headers,
            secrets: key_secrets(&key),
            ids: id_source.unwrap_or_else(|| Arc::new(DefaultIdSource)),
        };
        let spec = parts.spec(parts.hooks(), parts.extra_headers.clone());
        Ok(Self {
            parts: Arc::new(parts),
            spec: Arc::new(spec),
            family: family.name(),
            key_placement: key_placement.kind(),
        })
    }

    pub fn call_with<'a>(
        &'a self,
        request: &'a ClientRequest,
        options: &'a LiteLlmCallOptions,
        sink: &'a mut dyn StreamSink,
    ) -> BoxFuture<'a, LiteLlmCallOutcome> {
        Box::pin(async move {
            let slot = Arc::new(OnceLock::new());
            let hooks = self.parts.hooks().with_deployment_slot(Arc::clone(&slot));
            let spec = self
                .parts
                .spec(hooks, self.parts.extra_headers_for(options));
            let outcome = run(&spec, request, sink).await;
            LiteLlmCallOutcome::new(outcome, self.parts.deployment_id(&slot))
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
        &self.parts.settings
    }
}

impl fmt::Debug for LiteLlmClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LiteLlmClient")
            .field("endpoint", &format_args!("{}", self.parts.endpoint))
            .field("family", &self.family)
            .field("key_placement", &self.key_placement)
            .field("extra_headers", &self.parts.extra_headers.len())
            .finish_non_exhaustive()
    }
}
