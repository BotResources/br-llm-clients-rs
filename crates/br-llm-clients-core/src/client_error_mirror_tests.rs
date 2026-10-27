use std::collections::HashSet;
use std::mem::discriminant;

use crate::client_error::ClientError;
use crate::client_error_raw::RawClientError;
use crate::client_error_tests::every_case;

const CASES: usize = 27;

fn mirror(error: &ClientError) -> RawClientError {
    match error {
        ClientError::ModelNotFound => RawClientError::ModelNotFound {},
        ClientError::ModelNotAccessible => RawClientError::ModelNotAccessible {},
        ClientError::Authentication => RawClientError::Authentication {},
        ClientError::PermissionDenied => RawClientError::PermissionDenied {},
        ClientError::ContextTooLong { limit } => RawClientError::ContextTooLong { limit: *limit },
        ClientError::ContentBlocked => RawClientError::ContentBlocked {},
        ClientError::ReplyUnusable { ending } => RawClientError::ReplyUnusable {
            ending: ending.clone(),
        },
        ClientError::InvalidMessages { reason } => RawClientError::InvalidMessages {
            reason: reason.clone(),
        },
        ClientError::QuotaExceeded => RawClientError::QuotaExceeded {},
        ClientError::RateLimited { retry_after } => RawClientError::RateLimited {
            retry_after: *retry_after,
        },
        ClientError::Unavailable { retry_after } => RawClientError::Unavailable {
            retry_after: *retry_after,
        },
        ClientError::StreamTruncated => RawClientError::StreamTruncated {},
        ClientError::Transport => RawClientError::Transport {},
        ClientError::ReasoningNotSupported { level, reason } => {
            RawClientError::ReasoningNotSupported {
                level: *level,
                reason: reason.clone(),
            }
        }
        ClientError::StructuredOutputNotSupported => {
            RawClientError::StructuredOutputNotSupported {}
        }
        ClientError::StructuredOutputWithToolsNotSupported => {
            RawClientError::StructuredOutputWithToolsNotSupported {}
        }
        ClientError::ToolsNotSupported => RawClientError::ToolsNotSupported {},
        ClientError::CacheNotSupported => RawClientError::CacheNotSupported {},
        ClientError::ImageNotSupported => RawClientError::ImageNotSupported {},
        ClientError::ImageMediaTypeNotSupported { media_type } => {
            RawClientError::ImageMediaTypeNotSupported {
                media_type: *media_type,
            }
        }
        ClientError::LimitExceeded { kind, description } => RawClientError::LimitExceeded {
            kind: *kind,
            description: description.clone(),
        },
        ClientError::MaxOutputTokensTooLarge { limit } => {
            RawClientError::MaxOutputTokensTooLarge { limit: *limit }
        }
        ClientError::ToolSchemaRejected { tool, reason } => RawClientError::ToolSchemaRejected {
            tool: tool.clone(),
            reason: reason.clone(),
        },
        ClientError::OutputShapeRejected { reason } => RawClientError::OutputShapeRejected {
            reason: reason.clone(),
        },
        ClientError::MalformedResponse { detail } => RawClientError::MalformedResponse {
            detail: detail.clone(),
        },
        ClientError::ProviderRejected { status, code, body } => RawClientError::ProviderRejected {
            status: *status,
            code: code.clone(),
            body: body.clone(),
        },
        ClientError::ProviderFailed { status, code, body } => RawClientError::ProviderFailed {
            status: *status,
            code: code.clone(),
            body: body.clone(),
        },
    }
}

#[test]
fn given_every_case_when_mirrored_then_the_raw_case_converts_back_to_the_same_error() {
    for error in every_case() {
        assert_eq!(ClientError::from(mirror(&error)), error);
    }
}

#[test]
fn given_the_cases_listed_for_the_tests_when_counted_then_each_of_the_cases_is_there() {
    let kinds: HashSet<_> = every_case().iter().map(discriminant).collect();
    assert_eq!(kinds.len(), CASES);
}
