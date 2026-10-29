use std::fmt;
use std::sync::Arc;

use br_llm_clients_common_call::{CallSpec, Layered, run};
use br_llm_clients_common_http::{BearerAuthorizer, Endpoint, HttpClient};
use br_llm_clients_core::{
    BoxFuture, CallOutcome, Client, ClientRequest, ClientSettings, DefaultIdSource, StreamSink,
};

use crate::build_error::OpenRouterBuildError;
use crate::check_key::check_key;
use crate::config::OpenRouterConfig;
use crate::headers::{Attribution, request_headers};
use crate::hooks::OpenRouterHooks;
use crate::route::{base_endpoint, route_dialect, route_endpoint};

#[derive(Clone)]
pub struct OpenRouterClient {
    settings: ClientSettings,
    spec: Arc<CallSpec>,
    endpoint: Endpoint,
    family: &'static str,
    headers: usize,
}

impl OpenRouterClient {
    pub fn new(
        settings: ClientSettings,
        config: OpenRouterConfig,
        http: HttpClient,
    ) -> Result<Self, OpenRouterBuildError> {
        let OpenRouterConfig {
            key,
            family,
            capabilities,
            base,
            app_referer,
            app_title,
            extra_headers,
            id_source,
        } = config;
        check_key(&key)?;
        let base = base_endpoint(base)?;
        let attribution = Attribution {
            referer: app_referer.as_deref(),
            title: app_title.as_deref(),
        };
        let headers = request_headers(&base, &attribution, extra_headers)?;
        let endpoint = route_endpoint(&base, &family)?;
        let route = route_dialect(&settings, &family)?;
        let hooks = OpenRouterHooks::new(
            settings.clone(),
            capabilities,
            Arc::clone(&route),
            family.is_chat(),
        );
        let header_count = headers.len();
        let spec = CallSpec::new(
            Arc::new(Layered::new(route, hooks)),
            endpoint.clone(),
            Arc::new(BearerAuthorizer::new(&key)),
            http,
        )
        .with_extra_headers(headers)
        .with_secrets(vec![key])
        .with_id_source(id_source.unwrap_or_else(|| Arc::new(DefaultIdSource)));
        Ok(Self {
            settings,
            spec: Arc::new(spec),
            endpoint,
            family: family.name(),
            headers: header_count,
        })
    }
}

impl Client for OpenRouterClient {
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

impl fmt::Debug for OpenRouterClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenRouterClient")
            .field("endpoint", &format_args!("{}", self.endpoint))
            .field("family", &self.family)
            .field("headers", &self.headers)
            .finish_non_exhaustive()
    }
}
