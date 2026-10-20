use br_llm_clients_api_anthropic_messages::AnthropicMessages;
use br_llm_clients_api_openai_chat_completions::ChatCompletionsDialect;
use br_llm_clients_api_openai_responses::OpenAiResponses;
use br_llm_clients_common_call::Dialect;
use br_llm_clients_common_http::Endpoint;
use br_llm_clients_core::ClientSettings;

use crate::api_family::ApiFamily;
use crate::build_error::LiteLlmBuildError;

pub(crate) fn route_endpoint(
    base: &Endpoint,
    family: &ApiFamily,
) -> Result<Endpoint, LiteLlmBuildError> {
    family
        .route()
        .iter()
        .try_fold(base.clone(), |endpoint, segment| {
            endpoint.join_segment(segment)
        })
        .map_err(|_| LiteLlmBuildError::InvalidEndpoint {
            reason: "the endpoint of the route is longer than the limit of an endpoint".to_owned(),
        })
}

pub(crate) fn route_dialect(
    settings: &ClientSettings,
    family: &ApiFamily,
) -> Result<Box<dyn Dialect>, LiteLlmBuildError> {
    match family {
        ApiFamily::ChatCompletions(options) => {
            ChatCompletionsDialect::new(settings.clone(), options.clone())
                .map(|dialect| Box::new(dialect) as Box<dyn Dialect>)
                .map_err(|error| LiteLlmBuildError::InvalidDialectOptions {
                    reason: error.to_string(),
                })
        }
        ApiFamily::Responses(options) => Ok(Box::new(OpenAiResponses::new(settings, *options))),
        ApiFamily::Messages => Ok(Box::new(AnthropicMessages::new(settings))),
    }
}
