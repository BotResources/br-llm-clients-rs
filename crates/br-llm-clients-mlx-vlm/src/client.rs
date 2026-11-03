use std::fmt;
use std::sync::Arc;

use br_llm_clients_common_call::{CallSpec, Layered, run};
use br_llm_clients_common_http::{
    Authorizer, BearerAuthorizer, Endpoint, HttpClient, NoAuthorizer,
};
use br_llm_clients_core::{
    BoxFuture, CallOutcome, Client, ClientRequest, ClientSettings, DefaultIdSource, StreamSink,
};

use crate::build_error::MlxVlmBuildError;
use crate::check_key::check_key;
use crate::config::MlxVlmConfig;
use crate::extra_headers::check_extra_headers;
use crate::hooks::MlxVlmHooks;
use crate::options::chat_dialect;
use crate::route::route_endpoint;

#[derive(Clone)]
pub struct MlxVlmClient {
    settings: ClientSettings,
    spec: Arc<CallSpec>,
    endpoint: Endpoint,
    key: bool,
    extra_headers: usize,
    reasoning_carry_back: bool,
}

impl MlxVlmClient {
    pub fn new(
        settings: ClientSettings,
        config: MlxVlmConfig,
        http: HttpClient,
    ) -> Result<Self, MlxVlmBuildError> {
        let MlxVlmConfig {
            base,
            reasoning_table,
            capabilities,
            key,
            extra_body,
            extra_headers,
            reasoning_carry_back,
            id_source,
        } = config;
        if let Some(key) = &key {
            check_key(key)?;
        }
        check_extra_headers(&extra_headers, &base)?;
        let endpoint = route_endpoint(&base)?;
        let chat = chat_dialect(&settings, reasoning_table, extra_body, reasoning_carry_back)?;
        let hooks = MlxVlmHooks::new(settings.clone(), capabilities);
        let authorizer: Arc<dyn Authorizer> = match &key {
            Some(key) => Arc::new(BearerAuthorizer::new(key)),
            None => Arc::new(NoAuthorizer),
        };
        let extra_header_count = extra_headers.len();
        let spec = CallSpec::new(
            Arc::new(Layered::new(chat, hooks)),
            endpoint.clone(),
            authorizer,
            http,
        )
        .with_extra_headers(extra_headers)
        .with_secrets(key.iter().cloned().collect())
        .with_id_source(id_source.unwrap_or_else(|| Arc::new(DefaultIdSource)));
        Ok(Self {
            settings,
            spec: Arc::new(spec),
            endpoint,
            key: key.is_some(),
            extra_headers: extra_header_count,
            reasoning_carry_back,
        })
    }
}

impl Client for MlxVlmClient {
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

impl fmt::Debug for MlxVlmClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let key = if self.key { "set" } else { "none" };
        f.debug_struct("MlxVlmClient")
            .field("endpoint", &format_args!("{}", self.endpoint))
            .field("key", &key)
            .field("extra_headers", &self.extra_headers)
            .field("reasoning_carry_back", &self.reasoning_carry_back)
            .finish_non_exhaustive()
    }
}
