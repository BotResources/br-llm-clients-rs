use br_llm_messages::StopReason;

pub(crate) fn stop_reason(finish_reason: &str, refusal_seen: bool) -> StopReason {
    let mapped = match finish_reason {
        "stop" => StopReason::EndTurn,
        "length" => StopReason::MaxTokens,
        "tool_calls" => StopReason::AwaitingToolResults,
        "content_filter" => StopReason::ContentFilter,
        other => StopReason::Other {
            reason: other.to_owned(),
        },
    };
    match mapped {
        StopReason::EndTurn if refusal_seen => StopReason::Refusal,
        other => other,
    }
}
