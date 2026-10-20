use std::fmt;
use std::sync::Arc;

use br_llm_clients_api_openai_chat_completions::ChatCompletionsDialect;
use br_llm_clients_common_call::{CallSpec, run};
use br_llm_clients_common_http::{BearerAuthorizer, HttpClient};
use br_llm_clients_core::{
    BoxFuture, CallOutcome, Client, ClientRequest, ClientSettings, StreamSink,
};

use crate::build_error::KimiBuildError;
use crate::capabilities::capabilities;
use crate::config::KimiConfig;
use crate::credential::check_key;
use crate::dialect_options::dialect_options;
use crate::hooks::KimiHooks;
use crate::kimi_thinking::KimiThinking;
use crate::route::chat_route;

#[derive(Clone)]
pub struct KimiClient {
    settings: ClientSettings,
    thinking: Option<KimiThinking>,
    base: String,
    spec: Arc<CallSpec>,
}

impl KimiClient {
    pub fn new(
        settings: ClientSettings,
        config: KimiConfig,
        http: HttpClient,
    ) -> Result<Self, KimiBuildError> {
        check_key(config.key())?;
        let route = chat_route(config.base())?;
        let thinking = config
            .thinking()
            .cloned()
            .or_else(|| KimiThinking::of_model(settings.model().as_str()));
        let options = dialect_options(thinking.as_ref(), settings.reasoning());
        let inner = ChatCompletionsDialect::new(settings.clone(), options).map_err(|error| {
            KimiBuildError::InvalidDialectOptions {
                reason: error.to_string(),
            }
        })?;
        let hooks = KimiHooks::new(
            inner,
            settings.clone(),
            thinking.clone(),
            capabilities(thinking.as_ref()),
            config.key().clone(),
        );
        let spec = CallSpec::new(
            Arc::new(hooks),
            route,
            Arc::new(BearerAuthorizer::new(config.key())),
            http,
            settings.model().clone(),
        )
        .with_secrets(vec![config.key().clone()]);
        Ok(Self {
            base: config.base_text(),
            settings,
            thinking,
            spec: Arc::new(spec),
        })
    }

    pub fn thinking(&self) -> Option<KimiThinking> {
        self.thinking.clone()
    }
}

impl Client for KimiClient {
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

impl fmt::Debug for KimiClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KimiClient")
            .field("model", self.settings.model())
            .field("thinking", &self.thinking)
            .field("base", &self.base)
            .finish_non_exhaustive()
    }
}
