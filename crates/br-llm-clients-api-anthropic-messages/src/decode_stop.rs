use br_llm_clients_common_call::PayloadError;
use br_llm_messages::ResponseEnding;
use serde_json::Value;

use crate::decode_fields::{Fields, malformed};

pub(crate) fn map_stop_reason(reason: &str) -> Result<ResponseEnding, PayloadError> {
    Ok(match reason {
        "end_turn" => ResponseEnding::finished(),
        "max_tokens" => ResponseEnding::output_exhaustion(),
        "stop_sequence" => ResponseEnding::stop_sequence(),
        "tool_use" => ResponseEnding::tool_use(),
        "refusal" => ResponseEnding::refusal(),
        other => ResponseEnding::unclassified(other)
            .map_err(|_| malformed("the stop reason cannot be carried"))?,
    })
}

pub(crate) fn read_stop_reason(delta: &Fields) -> Result<Option<ResponseEnding>, PayloadError> {
    match delta.get("stop_reason") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(reason)) if !reason.is_empty() => map_stop_reason(reason).map(Some),
        Some(_) => Err(malformed(
            "the stop reason is empty or is neither text nor null",
        )),
    }
}
