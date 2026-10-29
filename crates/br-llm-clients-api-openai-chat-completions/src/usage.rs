use br_llm_clients_common_call::PayloadError;
use br_llm_messages::Usage;
use serde_json::{Map, Value};

use crate::json_fields::{count_field, object_field, present, wrong};

struct Counts {
    input: Option<u64>,
    output: Option<u64>,
    cache_read: Option<u64>,
    cache_creation: Option<u64>,
    thinking: Option<u64>,
}

pub(crate) fn read(
    chunk: &Map<String, Value>,
    cache_read_field: Option<&str>,
) -> Result<Option<Usage>, PayloadError> {
    let usage = match present(chunk, "usage") {
        None => return Ok(None),
        Some(Value::Object(usage)) => usage,
        Some(_) => return Err(wrong("usage", "an object")),
    };
    Ok(build(counts(usage, cache_read_field)?))
}

fn counts(
    usage: &Map<String, Value>,
    cache_read_field: Option<&str>,
) -> Result<Counts, PayloadError> {
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
    let cache_creation = detail(
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
    Ok(Counts {
        input,
        output,
        cache_read,
        cache_creation,
        thinking,
    })
}

fn build(counts: Counts) -> Option<Usage> {
    let mut usage = match (counts.input, counts.output) {
        (Some(input), Some(output)) => Usage::of_input_tokens(input).with_output_tokens(output),
        (Some(input), None) => Usage::of_input_tokens(input),
        (None, Some(output)) => Usage::of_output_tokens(output),
        (None, None) => return None,
    };
    if counts.input.is_some() {
        if let Some(cache_read) = counts.cache_read {
            usage = usage.with_cache_read_input_tokens(cache_read);
        }
        if let Some(cache_creation) = counts.cache_creation {
            usage = usage.with_cache_creation_input_tokens(cache_creation);
        }
    }
    if counts.output.is_some()
        && let Some(thinking) = counts.thinking
    {
        usage = usage.with_thinking_tokens(thinking);
    }
    Some(usage)
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
