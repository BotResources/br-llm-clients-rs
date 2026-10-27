use std::time::Duration;

use br_llm_messages::{MediaType, ResponseEnding, ToolName};

use crate::client_error::{ClientError, LimitKind};
use crate::reasoning::ReasoningLevel;
use crate::schema::present;

#[derive(serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum RawClientError {
    ModelNotFound {},
    ModelNotAccessible {},
    Authentication {},
    PermissionDenied {},
    ContextTooLong {
        #[serde(default, deserialize_with = "present")]
        limit: Option<u64>,
    },
    ContentBlocked {},
    ReplyUnusable {
        ending: ResponseEnding,
    },
    InvalidMessages {
        reason: String,
    },
    QuotaExceeded {},
    RateLimited {
        #[serde(default, deserialize_with = "present")]
        retry_after: Option<Duration>,
    },
    Unavailable {
        #[serde(default, deserialize_with = "present")]
        retry_after: Option<Duration>,
    },
    StreamTruncated {},
    Transport {},
    ReasoningNotSupported {
        level: ReasoningLevel,
        reason: String,
    },
    StructuredOutputNotSupported {},
    StructuredOutputWithToolsNotSupported {},
    ToolsNotSupported {},
    CacheNotSupported {},
    ImageNotSupported {},
    ImageMediaTypeNotSupported {
        media_type: MediaType,
    },
    LimitExceeded {
        kind: LimitKind,
        description: String,
    },
    MaxOutputTokensTooLarge {
        #[serde(default, deserialize_with = "present")]
        limit: Option<u64>,
    },
    ToolSchemaRejected {
        #[serde(default, deserialize_with = "present")]
        tool: Option<ToolName>,
        reason: String,
    },
    OutputShapeRejected {
        reason: String,
    },
    MalformedResponse {
        detail: String,
    },
    ProviderRejected {
        #[serde(default, deserialize_with = "present")]
        status: Option<u16>,
        #[serde(default, deserialize_with = "present")]
        code: Option<String>,
        body: String,
    },
    ProviderFailed {
        #[serde(default, deserialize_with = "present")]
        status: Option<u16>,
        #[serde(default, deserialize_with = "present")]
        code: Option<String>,
        body: String,
    },
}

impl From<RawClientError> for ClientError {
    fn from(raw: RawClientError) -> Self {
        match raw {
            RawClientError::ModelNotFound {} => ClientError::ModelNotFound,
            RawClientError::ModelNotAccessible {} => ClientError::ModelNotAccessible,
            RawClientError::Authentication {} => ClientError::Authentication,
            RawClientError::PermissionDenied {} => ClientError::PermissionDenied,
            RawClientError::ContextTooLong { limit } => ClientError::ContextTooLong { limit },
            RawClientError::ContentBlocked {} => ClientError::ContentBlocked,
            RawClientError::ReplyUnusable { ending } => ClientError::ReplyUnusable { ending },
            RawClientError::InvalidMessages { reason } => ClientError::InvalidMessages { reason },
            RawClientError::QuotaExceeded {} => ClientError::QuotaExceeded,
            RawClientError::RateLimited { retry_after } => ClientError::RateLimited { retry_after },
            RawClientError::Unavailable { retry_after } => ClientError::Unavailable { retry_after },
            RawClientError::StreamTruncated {} => ClientError::StreamTruncated,
            RawClientError::Transport {} => ClientError::Transport,
            RawClientError::ReasoningNotSupported { level, reason } => {
                ClientError::ReasoningNotSupported { level, reason }
            }
            RawClientError::StructuredOutputNotSupported {} => {
                ClientError::StructuredOutputNotSupported
            }
            RawClientError::StructuredOutputWithToolsNotSupported {} => {
                ClientError::StructuredOutputWithToolsNotSupported
            }
            RawClientError::ToolsNotSupported {} => ClientError::ToolsNotSupported,
            RawClientError::CacheNotSupported {} => ClientError::CacheNotSupported,
            RawClientError::ImageNotSupported {} => ClientError::ImageNotSupported,
            RawClientError::ImageMediaTypeNotSupported { media_type } => {
                ClientError::ImageMediaTypeNotSupported { media_type }
            }
            RawClientError::LimitExceeded { kind, description } => {
                ClientError::LimitExceeded { kind, description }
            }
            RawClientError::MaxOutputTokensTooLarge { limit } => {
                ClientError::MaxOutputTokensTooLarge { limit }
            }
            RawClientError::ToolSchemaRejected { tool, reason } => {
                ClientError::ToolSchemaRejected { tool, reason }
            }
            RawClientError::OutputShapeRejected { reason } => {
                ClientError::OutputShapeRejected { reason }
            }
            RawClientError::MalformedResponse { detail } => {
                ClientError::MalformedResponse { detail }
            }
            RawClientError::ProviderRejected { status, code, body } => {
                ClientError::ProviderRejected { status, code, body }
            }
            RawClientError::ProviderFailed { status, code, body } => {
                ClientError::ProviderFailed { status, code, body }
            }
        }
    }
}
