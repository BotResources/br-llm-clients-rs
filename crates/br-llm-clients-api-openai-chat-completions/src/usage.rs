use br_llm_messages::Usage;
use serde_json::{Map, Value};

use br_llm_clients_common_call::PayloadError;

use crate::json_fields::{count_field, object_field, present, wrong};

pub(crate) fn read(
    chunk: &Map<String, Value>,
    cache_read_field: Option<&str>,
) -> Result<Option<Usage>, PayloadError> {
    let usage = match present(chunk, "usage") {
        None => return Ok(None),
        Some(Value::Object(usage)) => usage,
        Some(_) => return Err(wrong("usage", "an object")),
    };
    let input = count_field(usage, "prompt_tokens", "usage.prompt_tokens")?;
    let output = count_field(usage, "completion_tokens", "usage.completion_tokens")?;
    let prompt_details = object_field(
        usage,
        "prompt_tokens_details",
        "usage.prompt_tokens_details",
    )?;
    let completion_details = object_field(
        usage,
        "completion_tokens_details",
        "usage.completion_tokens_details",
    )?;
    let cached = detail(
        prompt_details,
        "cached_tokens",
        "usage.prompt_tokens_details.cached_tokens",
    )?;
    let cache_write = detail(
        prompt_details,
        "cache_write_tokens",
        "usage.prompt_tokens_details.cache_write_tokens",
    )?;
    let thinking = detail(
        completion_details,
        "reasoning_tokens",
        "usage.completion_tokens_details.reasoning_tokens",
    )?;
    let cache_read = match (cached, cache_read_field) {
        (Some(cached), _) => Some(cached),
        (None, Some(field)) => count_field(usage, field, &format!("usage.{field}"))?,
        (None, None) => None,
    };
    let (Some(input_tokens), Some(output_tokens)) = (input, output) else {
        return Ok(None);
    };
    Ok(Some(Usage {
        input_tokens,
        output_tokens,
        cache_read_tokens: cache_read,
        cache_write_tokens: cache_write,
        thinking_tokens: thinking,
    }))
}

fn detail(
    details: Option<&Map<String, Value>>,
    name: &str,
    place: &str,
) -> Result<Option<u64>, PayloadError> {
    match details {
        Some(details) => count_field(details, name, place),
        None => Ok(None),
    }
}
