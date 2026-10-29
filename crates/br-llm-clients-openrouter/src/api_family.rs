use br_llm_clients_api_anthropic_messages::MessagesOptions;
use br_llm_clients_api_openai_chat_completions::ChatCompletionsOptions;
use br_llm_clients_api_openai_responses::Options;

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ApiFamily {
    ChatCompletions(ChatCompletionsOptions),
    Responses(Options),
    Messages(MessagesOptions),
}

impl ApiFamily {
    pub(crate) fn name(&self) -> &'static str {
        match self {
            ApiFamily::ChatCompletions(_) => "ChatCompletions",
            ApiFamily::Responses(_) => "Responses",
            ApiFamily::Messages(_) => "Messages",
        }
    }

    pub(crate) fn route(&self) -> &'static [&'static str] {
        match self {
            ApiFamily::ChatCompletions(_) => &["chat", "completions"],
            ApiFamily::Responses(_) => &["responses"],
            ApiFamily::Messages(_) => &["messages"],
        }
    }

    pub(crate) fn is_chat(&self) -> bool {
        matches!(self, ApiFamily::ChatCompletions(_))
    }
}
