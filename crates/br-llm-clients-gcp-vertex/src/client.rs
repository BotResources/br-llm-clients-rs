use std::fmt;
use std::sync::Arc;

use br_llm_clients_api_anthropic_messages::{
    AnthropicMessages, MessagesOptions, ModelPlacement, VersionPlacement,
};
use br_llm_clients_common_auth::{TokenAuthorizer, TokenCache};
use br_llm_clients_common_call::{CallSpec, HttpClient, Layered, Secret, SecretLog, run};
use br_llm_clients_core::{
    BoxFuture, CallOutcome, Client, ClientRequest, ClientSettings, StreamSink,
};

use crate::build_error::VertexBuildError;
use crate::capabilities;
use crate::config::VertexConfig;
use crate::credential::VertexCredential;
use crate::google_token::GoogleTokenSource;
use crate::hooks::VertexHooks;
use crate::location;
use crate::project;
use crate::route::predict_endpoint;

const VERTEX_VERSION: &str = "vertex-2023-10-16";

#[derive(Clone)]
pub struct VertexClient {
    spec: Arc<CallSpec>,
    settings: ClientSettings,
    project: String,
    location: String,
    base_override: bool,
}

struct Authorization {
    authorizer: TokenAuthorizer,
    secrets: Vec<Secret>,
    log: SecretLog,
}

impl VertexClient {
    pub fn new(
        settings: ClientSettings,
        config: VertexConfig,
        http: HttpClient,
    ) -> Result<Self, VertexBuildError> {
        let VertexConfig {
            project,
            location,
            credential,
            base,
            id_source,
        } = config;
        project::check(&project)?;
        location::check(&location)?;
        let base_override = base.is_some();
        let endpoint = predict_endpoint(&project, &location, settings.model().as_str(), base)?;
        let authorization = authorization(credential, &http);
        let options = MessagesOptions::new()
            .with_model_placement(ModelPlacement::Omitted)
            .with_version_placement(VersionPlacement::Body(VERTEX_VERSION.to_owned()));
        let dialect = Layered::new(
            Arc::new(AnthropicMessages::with_options(&settings, options)),
            VertexHooks::new(
                capabilities::of_model(settings.model().as_str()),
                settings.clone(),
            ),
        );
        let spec = CallSpec::new(
            Arc::new(dialect),
            endpoint,
            Arc::new(authorization.authorizer),
            http,
        )
        .with_secrets(authorization.secrets)
        .with_secret_log(authorization.log);
        let spec = match id_source {
            Some(ids) => spec.with_id_source(ids),
            None => spec,
        };
        Ok(Self {
            spec: Arc::new(spec),
            settings,
            project,
            location,
            base_override,
        })
    }
}

fn authorization(credential: VertexCredential, http: &HttpClient) -> Authorization {
    let log = SecretLog::new();
    match credential {
        VertexCredential::ServiceAccount(key) => {
            let source = GoogleTokenSource::new(key, http.clone(), log.clone());
            let cache = TokenCache::new(Arc::new(source));
            let secrets = cache.secrets();
            let observed = log.clone();
            let authorizer = TokenAuthorizer::bearer(cache)
                .with_token_observer(Arc::new(move |token: &Secret| observed.record(token)));
            Authorization {
                authorizer,
                secrets,
                log,
            }
        }
    }
}

impl Client for VertexClient {
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

impl fmt::Debug for VertexClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VertexClient")
            .field("model", self.settings.model())
            .field("project", &self.project)
            .field("location", &self.location)
            .field("base_override", &self.base_override)
            .finish()
    }
}
