use br_llm_messages::{StopReason, Usage};

use crate::error_info::ErrorInfo;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderBlock {
    Text,
    Thinking,
    RedactedThinking,
    Structured,
    ToolCall { name: String, id: Option<String> },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProviderEvent {
    BlockStart { key: String, block: ProviderBlock },
    TextDelta { key: String, text: String },
    ThinkingDelta { key: String, text: String },
    SignatureDelta { key: String, signature: String },
    StructuredDelta { key: String, json: String },
    ToolArgumentsDelta { key: String, json: String },
    RedactedThinkingData { key: String, data: String },
    BlockEnd { key: String },
    Stop(StopReason),
    Usage(Usage),
    Model(String),
    Error(ErrorInfo),
    Complete,
}
