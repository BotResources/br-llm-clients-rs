use std::time::Duration;

use br_llm_messages::{MediaType, ResponseEnding, ToolName};

use crate::client_error_raw::RawClientError;
use crate::reasoning::ReasoningLevel;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum LimitKind {
    Tools,
    SchemaDepth,
    ImageCount,
    ImageSize,
    Other,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", from = "RawClientError")]
#[non_exhaustive]
pub enum ClientError {
    #[error("the model does not exist")]
    ModelNotFound,
    #[error("the model is not accessible with these credentials")]
    ModelNotAccessible,
    #[error("authentication failed")]
    Authentication,
    #[error("the credentials do not allow this call")]
    PermissionDenied,
    #[error("the input is longer than the context allows")]
    ContextTooLong {
        #[serde(skip_serializing_if = "Option::is_none")]
        limit: Option<u64>,
    },
    #[error("the content was blocked")]
    ContentBlocked,
    #[error("the reply holds nothing usable (ending {ending:?})")]
    ReplyUnusable { ending: ResponseEnding },
    #[error("invalid messages: {reason}")]
    InvalidMessages { reason: String },
    #[error("the quota is exhausted")]
    QuotaExceeded,
    #[error("rate limited")]
    RateLimited {
        #[serde(skip_serializing_if = "Option::is_none")]
        retry_after: Option<Duration>,
    },
    #[error("the provider is unavailable")]
    Unavailable {
        #[serde(skip_serializing_if = "Option::is_none")]
        retry_after: Option<Duration>,
    },
    #[error("the response stream ended before the reply was complete")]
    StreamTruncated,
    #[error("the connection failed before the response headers")]
    Transport,
    #[error("reasoning level {level:?} is not supported: {reason}")]
    ReasoningNotSupported {
        level: ReasoningLevel,
        reason: String,
    },
    #[error("the model cannot give a structured reply")]
    StructuredOutputNotSupported,
    #[error("a structured reply cannot be combined with tools")]
    StructuredOutputWithToolsNotSupported,
    #[error("the model cannot use tools")]
    ToolsNotSupported,
    #[error("the model cannot cache")]
    CacheNotSupported,
    #[error("the model takes no image as input")]
    ImageNotSupported,
    #[error("the model does not take images of type {}", .media_type.as_str())]
    ImageMediaTypeNotSupported { media_type: MediaType },
    #[error("limit exceeded ({kind:?}): {description}")]
    LimitExceeded {
        kind: LimitKind,
        description: String,
    },
    #[error("the number of output tokens is above what the model allows")]
    MaxOutputTokensTooLarge {
        #[serde(skip_serializing_if = "Option::is_none")]
        limit: Option<u64>,
    },
    #[error("tool schema rejected: {reason}")]
    ToolSchemaRejected {
        #[serde(skip_serializing_if = "Option::is_none")]
        tool: Option<ToolName>,
        reason: String,
    },
    #[error("output shape rejected: {reason}")]
    OutputShapeRejected { reason: String },
    #[error("malformed response: {detail}")]
    MalformedResponse { detail: String },
    #[error("rejected by the provider{}: {body}", status_suffix(*.status))]
    ProviderRejected {
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<u16>,
        #[serde(skip_serializing_if = "Option::is_none")]
        code: Option<String>,
        body: String,
    },
    #[error("failed at the provider{}: {body}", status_suffix(*.status))]
    ProviderFailed {
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<u16>,
        #[serde(skip_serializing_if = "Option::is_none")]
        code: Option<String>,
        body: String,
    },
}

fn status_suffix(status: Option<u16>) -> String {
    status
        .map(|status| format!(" (HTTP {status})"))
        .unwrap_or_default()
}

impl ClientError {
    pub fn is_retryable(&self) -> bool {
        match self {
            ClientError::RateLimited { .. }
            | ClientError::Unavailable { .. }
            | ClientError::ProviderFailed { .. }
            | ClientError::Transport
            | ClientError::StreamTruncated => true,
            ClientError::ModelNotFound
            | ClientError::ModelNotAccessible
            | ClientError::Authentication
            | ClientError::PermissionDenied
            | ClientError::ContextTooLong { .. }
            | ClientError::ContentBlocked
            | ClientError::ReplyUnusable { .. }
            | ClientError::InvalidMessages { .. }
            | ClientError::QuotaExceeded
            | ClientError::ReasoningNotSupported { .. }
            | ClientError::StructuredOutputNotSupported
            | ClientError::StructuredOutputWithToolsNotSupported
            | ClientError::ToolsNotSupported
            | ClientError::CacheNotSupported
            | ClientError::ImageNotSupported
            | ClientError::ImageMediaTypeNotSupported { .. }
            | ClientError::LimitExceeded { .. }
            | ClientError::MaxOutputTokensTooLarge { .. }
            | ClientError::ToolSchemaRejected { .. }
            | ClientError::OutputShapeRejected { .. }
            | ClientError::MalformedResponse { .. }
            | ClientError::ProviderRejected { .. } => false,
        }
    }
}
