use std::fmt;
use std::sync::Arc;

use br_llm_clients_api_anthropic_messages::AnthropicMessages;
use br_llm_clients_common_call::{
    Authorizer, CallSpec, HttpClient, Layered, Secret, SecretLog, run,
};
use br_llm_clients_common_http::ApiKeyAuthorizer;
use br_llm_clients_core::{
    BoxFuture, CallOutcome, Client, ClientRequest, ClientSettings, StreamSink,
};

use crate::build_error::BedrockBuildError;
use crate::capabilities;
use crate::config::BedrockConfig;
use crate::credential::BedrockCredential;
use crate::hooks::BedrockHooks;
use crate::region;
use crate::route::messages_endpoint;
use crate::sigv4::Sigv4Authorizer;
use crate::visible::is_visible;

const API_KEY_HEADER: &str = "x-api-key";

#[derive(Clone)]
pub struct BedrockClient {
    spec: Arc<CallSpec>,
    settings: ClientSettings,
    region: String,
    base_override: bool,
}

struct Authorization {
    authorizer: Arc<dyn Authorizer>,
    secrets: Vec<Secret>,
    log: SecretLog,
}

impl BedrockClient {
    pub fn new(
        settings: ClientSettings,
        config: BedrockConfig,
        http: HttpClient,
    ) -> Result<Self, BedrockBuildError> {
        let BedrockConfig {
            region,
            credential,
            base,
            id_source,
        } = config;
        region::check(&region)?;
        check_credential(&credential)?;
        let base_override = base.is_some();
        let endpoint = messages_endpoint(&region, base)?;
        let authorization = authorization(credential, &region)?;
        let dialect = Layered::new(
            Arc::new(AnthropicMessages::new(&settings)),
            BedrockHooks::new(
                capabilities::of_model(settings.model().as_str()),
                settings.clone(),
            ),
        );
        let spec = CallSpec::new(Arc::new(dialect), endpoint, authorization.authorizer, http)
            .with_secrets(authorization.secrets)
            .with_secret_log(authorization.log);
        let spec = match id_source {
            Some(ids) => spec.with_id_source(ids),
            None => spec,
        };
        Ok(Self {
            spec: Arc::new(spec),
            settings,
            region,
            base_override,
        })
    }
}

fn check_credential(credential: &BedrockCredential) -> Result<(), BedrockBuildError> {
    match credential {
        BedrockCredential::ApiKey(key) if !is_visible(key.expose()) => {
            Err(BedrockBuildError::credential(
                "the API key holds a character outside the visible ASCII range",
            ))
        }
        BedrockCredential::ApiKey(_) | BedrockCredential::Sigv4(_) => Ok(()),
    }
}

fn authorization(
    credential: BedrockCredential,
    region: &str,
) -> Result<Authorization, BedrockBuildError> {
    let log = SecretLog::new();
    match credential {
        BedrockCredential::ApiKey(key) => {
            let authorizer = ApiKeyAuthorizer::new(API_KEY_HEADER, key.clone()).map_err(|_| {
                BedrockBuildError::credential("the API key authorizer cannot be built")
            })?;
            Ok(Authorization {
                authorizer: Arc::new(authorizer),
                secrets: vec![key],
                log,
            })
        }
        BedrockCredential::Sigv4(source) => Ok(Authorization {
            authorizer: Arc::new(Sigv4Authorizer::new(source, region.to_owned(), log.clone())),
            secrets: Vec::new(),
            log,
        }),
    }
}

impl Client for BedrockClient {
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

impl fmt::Debug for BedrockClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BedrockClient")
            .field("model", self.settings.model())
            .field("region", &self.region)
            .field("base_override", &self.base_override)
            .finish()
    }
}
