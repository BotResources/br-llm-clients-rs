use br_llm_clients_common_call::PayloadError;
use br_llm_messages::ResponseEnding;

pub(crate) fn ending_of(
    finish_reason: &str,
    refusal_seen: bool,
) -> Result<ResponseEnding, PayloadError> {
    let mapped = match finish_reason {
        "stop" if refusal_seen => ResponseEnding::refusal(),
        "stop" => ResponseEnding::finished(),
        "length" => ResponseEnding::output_exhaustion(),
        "tool_calls" => ResponseEnding::tool_use(),
        "content_filter" => ResponseEnding::filtering(),
        other => ResponseEnding::unclassified(other)
            .map_err(|_| PayloadError::new("the finish reason is empty"))?,
    };
    Ok(mapped)
}
