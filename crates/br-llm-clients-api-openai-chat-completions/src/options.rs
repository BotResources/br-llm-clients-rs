use serde_json::{Map, Value};

use crate::max_tokens_field::MaxTokensField;
use crate::options_error::OptionsError;
use crate::reasoning_table::ReasoningTable;
use crate::tool_only_content::ToolOnlyContent;

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
    max_tokens_field: MaxTokensField,
    reasoning_carry_back_field: Option<String>,
    tool_message_name: bool,
    omit_effort_above_off: bool,
    tool_only_content: ToolOnlyContent,
    empty_tool_result_text: Option<String>,
}

impl ChatCompletionsOptions {
    pub fn new(reasoning_table: ReasoningTable) -> Self {
        Self {
            reasoning_table,
            reasoning_field: None,
            cache_read_field: None,
            extra_body: Map::new(),
            max_tokens_field: MaxTokensField::MaxCompletionTokens,
            reasoning_carry_back_field: None,
            tool_message_name: false,
            omit_effort_above_off: false,
            tool_only_content: ToolOnlyContent::Omitted,
            empty_tool_result_text: None,
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

    pub fn with_max_tokens_field(mut self, field: MaxTokensField) -> Self {
        self.max_tokens_field = field;
        self
    }

    pub fn with_reasoning_carry_back_field(mut self, field: impl Into<String>) -> Self {
        self.reasoning_carry_back_field = Some(field.into());
        self
    }

    pub fn with_tool_message_name(mut self, on: bool) -> Self {
        self.tool_message_name = on;
        self
    }

    pub fn with_omit_effort_above_off(mut self, on: bool) -> Self {
        self.omit_effort_above_off = on;
        self
    }

    pub fn with_tool_only_content(mut self, content: ToolOnlyContent) -> Self {
        self.tool_only_content = content;
        self
    }

    pub fn with_empty_tool_result_text(mut self, text: impl Into<String>) -> Self {
        self.empty_tool_result_text = Some(text.into());
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

    pub(crate) fn max_tokens_field(&self) -> MaxTokensField {
        self.max_tokens_field
    }

    pub(crate) fn reasoning_carry_back_field(&self) -> Option<&str> {
        self.reasoning_carry_back_field.as_deref()
    }

    pub(crate) fn tool_message_name(&self) -> bool {
        self.tool_message_name
    }

    pub(crate) fn tool_only_content(&self) -> ToolOnlyContent {
        self.tool_only_content
    }

    pub(crate) fn empty_tool_result_text(&self) -> Option<&str> {
        self.empty_tool_result_text.as_deref()
    }

    pub(crate) fn check(&self) -> Result<(), OptionsError> {
        self.reasoning_table.check(self.omit_effort_above_off)?;
        if let Some(field) = &self.reasoning_field {
            check_reasoning_field(field)?;
        }
        if let Some(field) = &self.cache_read_field
            && field.trim().is_empty()
        {
            return Err(OptionsError::BlankCacheReadField);
        }
        if let Some(field) = &self.reasoning_carry_back_field {
            check_carry_back_field(field)?;
        }
        if let Some(text) = &self.empty_tool_result_text
            && text.trim().is_empty()
        {
            return Err(OptionsError::BlankEmptyToolResultText);
        }
        if let Some(member) = self
            .extra_body
            .keys()
            .find(|key| self.is_reserved_body_member(key))
        {
            return Err(OptionsError::ReservedExtraBodyMember {
                member: member.clone(),
            });
        }
        Ok(())
    }
}

impl ChatCompletionsOptions {
    fn is_reserved_body_member(&self, member: &str) -> bool {
        RESERVED_BODY_MEMBERS.contains(&member)
            || (self.max_tokens_field == MaxTokensField::MaxTokens && member == "max_tokens")
    }
}

fn check_carry_back_field(field: &str) -> Result<(), OptionsError> {
    if field.trim().is_empty() {
        return Err(OptionsError::BlankCarryBackField);
    }
    if RESERVED_REASONING_FIELDS.contains(&field) {
        return Err(OptionsError::ReservedCarryBackField {
            field: field.to_owned(),
        });
    }
    Ok(())
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
