use std::sync::Arc;

use br_llm_clients_api_openai_chat_completions::{
    ChatCompletionsDialect, ChatCompletionsOptions, MaxTokensField, ReasoningTable,
};
use br_llm_clients_common_call::Dialect;
use br_llm_clients_core::ClientSettings;
use serde_json::{Map, Value};

use crate::build_error::MlxVlmBuildError;

const REASONING_FIELD: &str = "reasoning_content";

pub(crate) fn chat_dialect(
    settings: &ClientSettings,
    reasoning_table: ReasoningTable,
    extra_body: Map<String, Value>,
    reasoning_carry_back: bool,
) -> Result<Arc<dyn Dialect>, MlxVlmBuildError> {
    let options = ChatCompletionsOptions::new(reasoning_table)
        .with_reasoning_field(REASONING_FIELD)
        .with_max_tokens_field(MaxTokensField::MaxTokens)
        .with_extra_body(extra_body);
    let options = if reasoning_carry_back {
        options.with_reasoning_carry_back_field(REASONING_FIELD)
    } else {
        options
    };
    ChatCompletionsDialect::new(settings.clone(), options)
        .map(|dialect| Arc::new(dialect) as Arc<dyn Dialect>)
        .map_err(|error| MlxVlmBuildError::InvalidDialectOptions {
            reason: error.to_string(),
        })
}
