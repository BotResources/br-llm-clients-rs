use serde_json::{Map, Value};

use crate::options_error::OptionsError;
use crate::reasoning_table::ReasoningTable;

const RESERVED_REASONING_FIELDS: [&str; 6] = [
    "content",
    "refusal",
    "tool_calls",
    "role",
    "audio",
    "function_call",
];

const RESERVED_BODY_MEMBERS: [&str; 8] = [
    "model",
    "messages",
    "stream",
    "stream_options",
    "max_completion_tokens",
    "reasoning_effort",
    "tools",
    "response_format",
];

#[derive(Debug, Clone, PartialEq)]
pub struct ChatCompletionsOptions {
    reasoning_table: ReasoningTable,
    reasoning_field: Option<String>,
    cache_read_field: Option<String>,
    extra_body: Map<String, Value>,
}

impl ChatCompletionsOptions {
    pub fn new(reasoning_table: ReasoningTable) -> Self {
        Self {
            reasoning_table,
            reasoning_field: None,
            cache_read_field: None,
            extra_body: Map::new(),
        }
    }

    pub fn with_reasoning_field(mut self, field: impl Into<String>) -> Self {
        self.reasoning_field = Some(field.into());
        self
    }

    pub fn with_cache_read_field(mut self, field: impl Into<String>) -> Self {
        self.cache_read_field = Some(field.into());
        self
    }

    pub fn with_extra_body(mut self, extra_body: Map<String, Value>) -> Self {
        self.extra_body = extra_body;
        self
    }

    pub(crate) fn reasoning_table(&self) -> &ReasoningTable {
        &self.reasoning_table
    }

    pub(crate) fn reasoning_field(&self) -> Option<&str> {
        self.reasoning_field.as_deref()
    }

    pub(crate) fn cache_read_field(&self) -> Option<&str> {
        self.cache_read_field.as_deref()
    }

    pub(crate) fn extra_body(&self) -> &Map<String, Value> {
        &self.extra_body
    }

    pub(crate) fn check(&self) -> Result<(), OptionsError> {
        self.reasoning_table.check()?;
        if let Some(field) = &self.reasoning_field {
            check_reasoning_field(field)?;
        }
        if let Some(field) = &self.cache_read_field
            && field.trim().is_empty()
        {
            return Err(OptionsError::BlankCacheReadField);
        }
        if let Some(member) = self
            .extra_body
            .keys()
            .find(|key| RESERVED_BODY_MEMBERS.contains(&key.as_str()))
        {
            return Err(OptionsError::ReservedExtraBodyMember {
                member: member.clone(),
            });
        }
        Ok(())
    }
}

fn check_reasoning_field(field: &str) -> Result<(), OptionsError> {
    if field.trim().is_empty() {
        return Err(OptionsError::BlankReasoningField);
    }
    if RESERVED_REASONING_FIELDS.contains(&field) {
        return Err(OptionsError::ReservedReasoningField {
            field: field.to_owned(),
        });
    }
    Ok(())
}
