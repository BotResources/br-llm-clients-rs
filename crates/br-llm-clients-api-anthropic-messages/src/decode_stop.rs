use br_llm_clients_common_call::PayloadError;
use br_llm_messages::StopReason;
use serde_json::Value;

use crate::decode_fields::{Fields, malformed};

pub(crate) fn map_stop_reason(reason: &str) -> StopReason {
    match reason {
        "end_turn" => StopReason::EndTurn,
        "max_tokens" => StopReason::MaxTokens,
        "stop_sequence" => StopReason::StopSequence,
        "tool_use" => StopReason::AwaitingToolResults,
        "refusal" => StopReason::Refusal,
        other => StopReason::Other {
            reason: other.to_owned(),
        },
    }
}

pub(crate) fn read_stop_reason(delta: &Fields) -> Result<Option<StopReason>, PayloadError> {
    match delta.get("stop_reason") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(reason)) if !reason.is_empty() => Ok(Some(map_stop_reason(reason))),
        Some(_) => Err(malformed(
            "the stop reason is empty or is neither text nor null",
        )),
    }
}
