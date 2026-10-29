use std::sync::Arc;

use br_llm_clients_api_anthropic_messages::AnthropicMessages;
use br_llm_clients_api_openai_chat_completions::ChatCompletionsDialect;
use br_llm_clients_api_openai_responses::OpenAiResponses;
use br_llm_clients_common_call::Dialect;
use br_llm_clients_common_http::Endpoint;
use br_llm_clients_core::ClientSettings;

use crate::api_family::ApiFamily;
use crate::build_error::OpenRouterBuildError;
use crate::config::DEFAULT_BASE;

fn too_long() -> OpenRouterBuildError {
    OpenRouterBuildError::InvalidEndpoint {
        reason: "the endpoint of the route is longer than the limit of an endpoint".to_owned(),
    }
}

pub(crate) fn base_endpoint(base: Option<Endpoint>) -> Result<Endpoint, OpenRouterBuildError> {
    match base {
        Some(base) => Ok(base),
        None => Endpoint::https(DEFAULT_BASE).map_err(|_| OpenRouterBuildError::InvalidEndpoint {
            reason: "the default base of OpenRouter cannot be made".to_owned(),
        }),
    }
}

pub(crate) fn route_endpoint(
    base: &Endpoint,
    family: &ApiFamily,
) -> Result<Endpoint, OpenRouterBuildError> {
    family
        .route()
        .iter()
        .try_fold(base.clone(), |endpoint, segment| {
            endpoint.join_segment(segment)
        })
        .map_err(|_| too_long())
}

pub(crate) fn route_dialect(
    settings: &ClientSettings,
    family: &ApiFamily,
) -> Result<Arc<dyn Dialect>, OpenRouterBuildError> {
    match family {
        ApiFamily::ChatCompletions(options) => {
            ChatCompletionsDialect::new(settings.clone(), options.clone())
                .map(|dialect| Arc::new(dialect) as Arc<dyn Dialect>)
                .map_err(|error| OpenRouterBuildError::InvalidDialectOptions {
                    reason: error.to_string(),
                })
        }
        ApiFamily::Responses(options) => {
            Ok(Arc::new(OpenAiResponses::new(settings, options.clone())))
        }
        ApiFamily::Messages(options) => Ok(Arc::new(AnthropicMessages::with_options(
            settings,
            options.clone(),
        ))),
    }
}
