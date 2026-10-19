use br_llm_clients_common_call::PayloadError;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EventType {
    Created,
    ItemAdded,
    PartAdded,
    TextDelta,
    RefusalDelta,
    SummaryDelta,
    ReasoningDelta,
    ArgumentsDelta,
    ItemDone,
    Error,
    Completed,
    Incomplete,
    Failed,
}

impl EventType {
    pub(super) fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "response.created" => Self::Created,
            "response.output_item.added" => Self::ItemAdded,
            "response.content_part.added" => Self::PartAdded,
            "response.output_text.delta" => Self::TextDelta,
            "response.refusal.delta" => Self::RefusalDelta,
            "response.reasoning_summary_text.delta" => Self::SummaryDelta,
            "response.reasoning_text.delta" => Self::ReasoningDelta,
            "response.function_call_arguments.delta" => Self::ArgumentsDelta,
            "response.output_item.done" => Self::ItemDone,
            "error" => Self::Error,
            "response.completed" => Self::Completed,
            "response.incomplete" => Self::Incomplete,
            "response.failed" => Self::Failed,
            _ => return None,
        })
    }

    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Created => "response.created",
            Self::ItemAdded => "response.output_item.added",
            Self::PartAdded => "response.content_part.added",
            Self::TextDelta => "response.output_text.delta",
            Self::RefusalDelta => "response.refusal.delta",
            Self::SummaryDelta => "response.reasoning_summary_text.delta",
            Self::ReasoningDelta => "response.reasoning_text.delta",
            Self::ArgumentsDelta => "response.function_call_arguments.delta",
            Self::ItemDone => "response.output_item.done",
            Self::Error => "error",
            Self::Completed => "response.completed",
            Self::Incomplete => "response.incomplete",
            Self::Failed => "response.failed",
        }
    }

    pub(super) fn fail(self, problem: &str) -> PayloadError {
        PayloadError::new(format!("{}: {problem}", self.name()))
    }

    pub(super) fn index(self, value: &Value, key: &str) -> Result<u64, PayloadError> {
        value
            .get(key)
            .and_then(Value::as_u64)
            .ok_or_else(|| self.fail(&format!("{key} is missing or is not a whole number")))
    }

    pub(super) fn text<'a>(self, value: &'a Value, key: &str) -> Result<&'a str, PayloadError> {
        value
            .get(key)
            .and_then(Value::as_str)
            .ok_or_else(|| self.fail(&format!("{key} is missing or is not a text")))
    }

    pub(super) fn optional_text<'a>(
        self,
        value: &'a Value,
        key: &str,
    ) -> Result<Option<&'a str>, PayloadError> {
        match value.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(text)) => Ok(Some(text.as_str())),
            Some(_) => Err(self.fail(&format!("{key} is not a text"))),
        }
    }

    pub(super) fn object<'a>(self, value: &'a Value, key: &str) -> Result<&'a Value, PayloadError> {
        value
            .get(key)
            .filter(|found| found.is_object())
            .ok_or_else(|| self.fail(&format!("{key} is missing or is not an object")))
    }

    pub(super) fn optional_array<'a>(
        self,
        value: &'a Value,
        key: &str,
    ) -> Result<&'a [Value], PayloadError> {
        match value.get(key) {
            None | Some(Value::Null) => Ok(&[]),
            Some(Value::Array(items)) => Ok(items.as_slice()),
            Some(_) => Err(self.fail(&format!("{key} is not a list"))),
        }
    }
}
