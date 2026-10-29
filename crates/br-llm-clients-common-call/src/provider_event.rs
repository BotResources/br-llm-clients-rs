use br_llm_messages::{ReplayFormat, ResponseEnding, Usage};

use crate::error_info::ErrorInfo;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProviderBlock {
    Text,
    Thinking,
    RedactedThinking,
    Structured,
    ToolCall { name: String, id: Option<String> },
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ProviderEvent {
    BlockStart {
        key: String,
        block: ProviderBlock,
    },
    TextDelta {
        key: String,
        text: String,
    },
    ThinkingDelta {
        key: String,
        text: String,
    },
    SignatureDelta {
        key: String,
        format: ReplayFormat,
        signature: String,
    },
    StructuredDelta {
        key: String,
        json: String,
    },
    ToolArgumentsDelta {
        key: String,
        json: String,
    },
    RedactedThinkingData {
        key: String,
        format: ReplayFormat,
        data: String,
    },
    BlockEnd {
        key: String,
    },
    Stop(ResponseEnding),
    Usage(Usage),
    Model(String),
    ResponseId(String),
    Error(ErrorInfo),
    Complete,
}
