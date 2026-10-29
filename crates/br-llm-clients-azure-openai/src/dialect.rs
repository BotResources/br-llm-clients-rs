use std::sync::Arc;

use br_llm_clients_api_openai_chat_completions::ChatCompletionsDialect;
use br_llm_clients_api_openai_responses::OpenAiResponses;
use br_llm_clients_common_call::Dialect;
use br_llm_clients_core::ClientSettings;

use crate::api::AzureOpenAiApi;
use crate::build_error::AzureOpenAiBuildError;

pub(crate) fn route_dialect(
    settings: &ClientSettings,
    api: AzureOpenAiApi,
) -> Result<Arc<dyn Dialect>, AzureOpenAiBuildError> {
    match api {
        AzureOpenAiApi::ChatCompletions(options) => {
            ChatCompletionsDialect::new(settings.clone(), options)
                .map(|dialect| Arc::new(dialect) as Arc<dyn Dialect>)
                .map_err(|error| AzureOpenAiBuildError::InvalidDialectOptions {
                    reason: error.to_string(),
                })
        }
        AzureOpenAiApi::Responses(options) => Ok(Arc::new(OpenAiResponses::new(settings, options))),
    }
}
