use std::fmt;

use br_llm_clients_api_openai_chat_completions::ChatCompletionsOptions;
use br_llm_clients_api_openai_responses::Options;

#[derive(Clone, PartialEq)]
#[non_exhaustive]
pub enum AzureOpenAiApi {
    ChatCompletions(ChatCompletionsOptions),
    Responses(Options),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Route {
    ChatCompletions,
    Responses,
}

impl AzureOpenAiApi {
    pub(crate) fn route(&self) -> Route {
        match self {
            Self::ChatCompletions(_) => Route::ChatCompletions,
            Self::Responses(_) => Route::Responses,
        }
    }
}

impl fmt::Debug for AzureOpenAiApi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ChatCompletions(_) => f.debug_tuple("ChatCompletions").finish_non_exhaustive(),
            Self::Responses(options) => f.debug_tuple("Responses").field(options).finish(),
        }
    }
}
