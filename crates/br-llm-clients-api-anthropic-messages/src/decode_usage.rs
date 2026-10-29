use br_llm_clients_common_call::PayloadError;
use br_llm_messages::Usage;
use serde_json::Value;

use crate::decode_fields::{Fields, malformed};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct UsageRecord {
    input: Option<u64>,
    cache_read: Option<u64>,
    cache_creation: Option<u64>,
    output: Option<u64>,
    thinking: Option<u64>,
}

impl UsageRecord {
    pub(crate) fn merge(&mut self, usage: Option<&Value>) -> Result<(), PayloadError> {
        let fields = match usage {
            None | Some(Value::Null) => return Ok(()),
            Some(Value::Object(fields)) => fields,
            Some(_) => return Err(malformed("the usage is neither an object nor null")),
        };
        replace(&mut self.input, fields, "input_tokens")?;
        replace(&mut self.cache_read, fields, "cache_read_input_tokens")?;
        replace(
            &mut self.cache_creation,
            fields,
            "cache_creation_input_tokens",
        )?;
        replace(&mut self.output, fields, "output_tokens")?;
        match fields.get("output_tokens_details") {
            None | Some(Value::Null) => Ok(()),
            Some(Value::Object(details)) => replace(&mut self.thinking, details, "thinking_tokens"),
            Some(_) => Err(malformed(
                "the usage field output_tokens_details is neither an object nor null",
            )),
        }
    }

    fn input_count(&self) -> Result<Option<u64>, PayloadError> {
        let parts = [self.input, self.cache_read, self.cache_creation];
        if parts.iter().all(Option::is_none) {
            return Ok(None);
        }
        parts
            .iter()
            .try_fold(0u64, |sum, part| sum.checked_add(part.unwrap_or(0)))
            .map(Some)
            .ok_or_else(|| malformed("the input token count overflows"))
    }

    pub(crate) fn usage(&self) -> Result<Option<Usage>, PayloadError> {
        let input = self.input_count()?;
        let with_input = input.map(|count| {
            let usage = Usage::of_input_tokens(count);
            let usage = match self.cache_read {
                Some(count) => usage.with_cache_read_input_tokens(count),
                None => usage,
            };
            match self.cache_creation {
                Some(count) => usage.with_cache_creation_input_tokens(count),
                None => usage,
            }
        });
        let Some(output) = self.output else {
            return Ok(with_input);
        };
        let usage = match with_input {
            Some(usage) => usage.with_output_tokens(output),
            None => Usage::of_output_tokens(output),
        };
        Ok(Some(match self.thinking {
            Some(count) => usage.with_thinking_tokens(count),
            None => usage,
        }))
    }
}

fn replace(slot: &mut Option<u64>, fields: &Fields, name: &str) -> Result<(), PayloadError> {
    match fields.get(name) {
        None | Some(Value::Null) => Ok(()),
        Some(value) => {
            *slot = Some(value.as_u64().ok_or_else(|| {
                malformed(format!(
                    "the usage field {name} is not a non-negative integer"
                ))
            })?);
            Ok(())
        }
    }
}
