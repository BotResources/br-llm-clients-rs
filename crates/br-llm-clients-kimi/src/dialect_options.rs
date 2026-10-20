use br_llm_clients_api_openai_chat_completions::{
    ChatCompletionsOptions, Map, ReasoningEffort, ReasoningRule, ReasoningTable, Value,
};
use br_llm_clients_core::ReasoningLevel;

use crate::kimi_thinking::KimiThinking;

pub(crate) fn dialect_options(
    kind: Option<&KimiThinking>,
    level: ReasoningLevel,
) -> ChatCompletionsOptions {
    ChatCompletionsOptions::new(reasoning_table(kind))
        .with_reasoning_field("reasoning_content")
        .with_cache_read_field("cached_tokens")
        .with_extra_body(extra_body(kind, level))
}

fn reasoning_table(kind: Option<&KimiThinking>) -> ReasoningTable {
    let send = |effort| ReasoningRule::Send(effort);
    match kind {
        Some(KimiThinking::EffortScale) => ReasoningTable::new(
            ReasoningRule::Refuse,
            send(ReasoningEffort::Low),
            send(ReasoningEffort::High),
            send(ReasoningEffort::High),
            send(ReasoningEffort::Max),
            send(ReasoningEffort::Max),
        ),
        Some(KimiThinking::ThinkingSwitch) => above_off(ReasoningRule::Omit),
        Some(KimiThinking::AlwaysThinking) => above_off(ReasoningRule::Refuse),
        None => ReasoningTable::new(
            ReasoningRule::Refuse,
            ReasoningRule::Refuse,
            ReasoningRule::Refuse,
            ReasoningRule::Refuse,
            ReasoningRule::Refuse,
            ReasoningRule::Refuse,
        ),
    }
}

fn above_off(off: ReasoningRule) -> ReasoningTable {
    let high = ReasoningRule::Send(ReasoningEffort::High);
    ReasoningTable::new(off, high, high, high, high, high)
}

fn extra_body(kind: Option<&KimiThinking>, level: ReasoningLevel) -> Map<String, Value> {
    let mut body = Map::new();
    let thinking = match kind {
        Some(KimiThinking::ThinkingSwitch) if level == ReasoningLevel::Off => "disabled",
        Some(KimiThinking::ThinkingSwitch | KimiThinking::AlwaysThinking) => "enabled",
        Some(KimiThinking::EffortScale) | None => return body,
    };
    let mut member = Map::new();
    member.insert("type".to_owned(), Value::from(thinking));
    body.insert("thinking".to_owned(), Value::Object(member));
    body
}
