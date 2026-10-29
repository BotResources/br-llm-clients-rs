use std::fmt;
use std::sync::Arc;

use br_llm_clients_common_call::{CallSpec, HttpClient, Layered, run};
use br_llm_clients_core::{
    BoxFuture, CallOutcome, Client, ClientRequest, ClientSettings, StreamSink,
};

use crate::build_error::AzureOpenAiBuildError;
use crate::config::AzureOpenAiConfig;
use crate::dialect::route_dialect;
use crate::hooks::AzureOpenAiHooks;
use crate::route::route_endpoint;
use crate::validate;

#[derive(Clone)]
pub struct AzureOpenAiClient {
    spec: Arc<CallSpec>,
    settings: ClientSettings,
    endpoint: String,
}

impl AzureOpenAiClient {
    pub fn new(
        settings: ClientSettings,
        config: AzureOpenAiConfig,
        http: HttpClient,
    ) -> Result<Self, AzureOpenAiBuildError> {
        let AzureOpenAiConfig {
            endpoint,
            key,
            api,
            version,
            capabilities,
            id_source,
        } = config;
        let authorizer = validate::key_authorizer(&key)?;
        let api_version = validate::api_version(&version)?;
        let route = api.route();
        let endpoint = route_endpoint(endpoint, route, api_version, settings.model().as_str())?;
        let dialect = route_dialect(&settings, api)?;
        let endpoint_text = endpoint.to_string();
        let layered = Layered::new(
            dialect,
            AzureOpenAiHooks::new(capabilities, settings.clone(), route),
        );
        let spec = CallSpec::new(Arc::new(layered), endpoint, Arc::new(authorizer), http)
            .with_secrets(vec![key]);
        let spec = match id_source {
            Some(ids) => spec.with_id_source(ids),
            None => spec,
        };
        Ok(Self {
            spec: Arc::new(spec),
            settings,
            endpoint: endpoint_text,
        })
    }
}

impl Client for AzureOpenAiClient {
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

impl fmt::Debug for AzureOpenAiClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AzureOpenAiClient")
            .field("deployment", self.settings.model())
            .field("endpoint", &self.endpoint)
            .finish_non_exhaustive()
    }
}
