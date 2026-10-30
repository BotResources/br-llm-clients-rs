use std::fmt;
use std::sync::Arc;

use br_llm_clients_api_openai_responses::OpenAiResponses;
use br_llm_clients_common_call::{CallSpec, HttpClient, Layered, run};
use br_llm_clients_common_http::BearerAuthorizer;
use br_llm_clients_core::{
    BoxFuture, CallOutcome, Client, ClientRequest, ClientSettings, StreamSink,
};

use crate::build_error::OpenAiBuildError;
use crate::config::OpenAiConfig;
use crate::hooks::OpenAiHooks;
use crate::options::{OptionInputs, dialect_options};
use crate::route::responses_endpoint;
use crate::validate;

const ORGANIZATION_HEADER: &str = "OpenAI-Organization";
const PROJECT_HEADER: &str = "OpenAI-Project";

#[derive(Clone)]
pub struct OpenAiClient {
    spec: Arc<CallSpec>,
    settings: ClientSettings,
    endpoint: String,
}

impl OpenAiClient {
    pub fn new(
        settings: ClientSettings,
        config: OpenAiConfig,
        http: HttpClient,
    ) -> Result<Self, OpenAiBuildError> {
        let OpenAiConfig {
            key,
            profile,
            base,
            organization,
            project,
            prompt_cache_key,
            service_tier,
            cache_mode,
            extra_body,
            id_source,
        } = config;
        validate::credential(&key)?;
        validate::header_value(organization.as_deref(), |reason| {
            OpenAiBuildError::InvalidOrganization { reason }
        })?;
        validate::header_value(project.as_deref(), |reason| {
            OpenAiBuildError::InvalidProject { reason }
        })?;
        let endpoint = responses_endpoint(base)?;
        let options = dialect_options(OptionInputs {
            profile: &profile,
            prompt_cache_key,
            service_tier,
            cache_mode,
            extra_body,
        })?;
        let endpoint_text = endpoint.to_string();
        let mut headers = Vec::new();
        if let Some(value) = organization {
            headers.push((ORGANIZATION_HEADER.to_owned(), value));
        }
        if let Some(value) = project {
            headers.push((PROJECT_HEADER.to_owned(), value));
        }
        let dialect = Layered::new(
            Arc::new(OpenAiResponses::new(&settings, options)),
            OpenAiHooks::new(profile.into_capabilities(), settings.clone()),
        );
        let spec = CallSpec::new(
            Arc::new(dialect),
            endpoint,
            Arc::new(BearerAuthorizer::new(&key)),
            http,
        )
        .with_extra_headers(headers)
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

impl Client for OpenAiClient {
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

impl fmt::Debug for OpenAiClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenAiClient")
            .field("model", self.settings.model())
            .field("endpoint", &self.endpoint)
            .finish_non_exhaustive()
    }
}
