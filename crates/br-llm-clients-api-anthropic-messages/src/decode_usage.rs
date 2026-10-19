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
        replace(&mut self.output, fields, "output_tokens")
    }

    pub(crate) fn complete(&self) -> Result<Option<Usage>, PayloadError> {
        let (Some(input), Some(output)) = (self.input, self.output) else {
            return Ok(None);
        };
        let total_input = input
            .checked_add(self.cache_read.unwrap_or(0))
            .and_then(|sum| sum.checked_add(self.cache_creation.unwrap_or(0)))
            .ok_or_else(|| malformed("the input token count overflows"))?;
        Ok(Some(Usage {
            input_tokens: total_input,
            output_tokens: output,
            cache_read_tokens: self.cache_read,
            cache_write_tokens: self.cache_creation,
            thinking_tokens: None,
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
