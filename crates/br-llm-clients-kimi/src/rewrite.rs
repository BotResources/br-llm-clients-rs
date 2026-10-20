use br_llm_clients_common_call::EncodedRequest;
use br_llm_clients_core::{ClientError, ClientRequest, ClientSettings, ReasoningLevel};
use serde_json::Value;

use crate::kimi_thinking::KimiThinking;
use crate::rewrite_names::name_tool_messages;
use crate::rewrite_reasoning::send_back_reasoning;

pub(crate) fn rewrite(
    encoded: &EncodedRequest,
    request: &ClientRequest,
    settings: &ClientSettings,
    kind: &KimiThinking,
) -> Result<EncodedRequest, ClientError> {
    let mut body: Value = serde_json::from_slice(encoded.body()).map_err(|_| unreadable())?;
    let members = body.as_object_mut().ok_or_else(unreadable)?;
    if !sends_effort(kind) {
        members.remove("reasoning_effort");
    }
    let messages = members
        .get_mut("messages")
        .and_then(Value::as_array_mut)
        .ok_or_else(unreadable)?;
    name_tool_messages(messages, request);
    send_back_reasoning(
        messages,
        request,
        thinking_is_on(kind, settings.reasoning()),
    )?;
    let bytes = serde_json::to_vec(&body).map_err(|_| unreadable())?;
    let mut rewritten = EncodedRequest::new(bytes);
    for (name, value) in encoded.headers() {
        rewritten = rewritten.with_header(name.clone(), value.clone());
    }
    Ok(rewritten)
}

fn sends_effort(kind: &KimiThinking) -> bool {
    match kind {
        KimiThinking::EffortScale => true,
        KimiThinking::ThinkingSwitch | KimiThinking::AlwaysThinking => false,
    }
}

fn thinking_is_on(kind: &KimiThinking, level: ReasoningLevel) -> bool {
    match kind {
        KimiThinking::EffortScale => true,
        KimiThinking::ThinkingSwitch | KimiThinking::AlwaysThinking => level != ReasoningLevel::Off,
    }
}

fn unreadable() -> ClientError {
    ClientError::InvalidMessages {
        reason: "the body of the dialect cannot be read".to_owned(),
    }
}
