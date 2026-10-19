use br_llm_clients_common_call::PayloadError;
use br_llm_messages::Usage;
use serde_json::Value;

use super::event::EventType;

fn count(event: EventType, holder: &Value, key: &str) -> Result<Option<u64>, PayloadError> {
    match holder.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(found) => found
            .as_u64()
            .map(Some)
            .ok_or_else(|| event.fail(&format!("{key} is not a whole number from 0 up"))),
    }
}

fn details<'a>(
    event: EventType,
    usage: &'a Value,
    key: &str,
) -> Result<Option<&'a Value>, PayloadError> {
    match usage.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(found) if found.is_object() => Ok(Some(found)),
        Some(_) => Err(event.fail(&format!("{key} is not an object"))),
    }
}

fn detail_count(
    event: EventType,
    usage: &Value,
    group: &str,
    key: &str,
) -> Result<Option<u64>, PayloadError> {
    match details(event, usage, group)? {
        Some(found) => count(event, found, key),
        None => Ok(None),
    }
}

pub(super) fn usage_of(event: EventType, response: &Value) -> Result<Option<Usage>, PayloadError> {
    let usage = match response.get("usage") {
        None | Some(Value::Null) => return Ok(None),
        Some(found) if found.is_object() => found,
        Some(_) => return Err(event.fail("usage is not an object")),
    };
    let input = count(event, usage, "input_tokens")?;
    let output = count(event, usage, "output_tokens")?;
    let cache_read_tokens = detail_count(event, usage, "input_tokens_details", "cached_tokens")?;
    let cache_write_tokens =
        detail_count(event, usage, "input_tokens_details", "cache_write_tokens")?;
    let thinking_tokens = detail_count(event, usage, "output_tokens_details", "reasoning_tokens")?;
    let (Some(input_tokens), Some(output_tokens)) = (input, output) else {
        return Ok(None);
    };
    Ok(Some(Usage {
        input_tokens,
        output_tokens,
        cache_read_tokens,
        cache_write_tokens,
        thinking_tokens,
    }))
}
