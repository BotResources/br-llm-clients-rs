use std::time::Duration;

use br_llm_messages::{ImageMime, StopReason, ToolName};

use crate::client_error::{ClientError, LimitKind};
use crate::reasoning::ReasoningLevel;

fn every_case() -> Vec<ClientError> {
    vec![
        ClientError::ModelNotFound,
        ClientError::ModelNotAccessible,
        ClientError::Authentication,
        ClientError::PermissionDenied,
        ClientError::ContextTooLong {
            limit: Some(200_000),
        },
        ClientError::ContextTooLong { limit: None },
        ClientError::ContentBlocked,
        ClientError::ReplyUnusable {
            stop_reason: StopReason::MaxTokens,
        },
        ClientError::InvalidMessages {
            reason: "no".to_owned(),
        },
        ClientError::QuotaExceeded,
        ClientError::RateLimited {
            retry_after: Some(Duration::from_secs(3)),
        },
        ClientError::RateLimited { retry_after: None },
        ClientError::Unavailable {
            retry_after: Some(Duration::from_millis(1500)),
        },
        ClientError::StreamTruncated,
        ClientError::Transport,
        ClientError::ReasoningNotSupported {
            level: ReasoningLevel::Off,
            reason: "always on".to_owned(),
        },
        ClientError::StructuredOutputNotSupported,
        ClientError::StructuredOutputWithToolsNotSupported,
        ClientError::ToolsNotSupported,
        ClientError::CacheNotSupported,
        ClientError::ImageNotSupported,
        ClientError::ImageMediaTypeNotSupported {
            media_type: ImageMime::Gif,
        },
        ClientError::LimitExceeded {
            kind: LimitKind::Tools,
            description: "at most 128 tools".to_owned(),
        },
        ClientError::MaxOutputTokensTooLarge {
            limit: Some(64_000),
        },
        ClientError::ToolSchemaRejected {
            tool: Some(ToolName::new("search").unwrap()),
            reason: "bad name".to_owned(),
        },
        ClientError::ToolSchemaRejected {
            tool: None,
            reason: "bad schema".to_owned(),
        },
        ClientError::OutputShapeRejected {
            reason: "bad shape".to_owned(),
        },
        ClientError::MalformedResponse {
            detail: "garbled".to_owned(),
        },
        ClientError::ProviderRejected {
            status: Some(422),
            code: Some("invalid".to_owned()),
            body: "{}".to_owned(),
        },
        ClientError::ProviderRejected {
            status: None,
            code: None,
            body: String::new(),
        },
        ClientError::ProviderFailed {
            status: Some(500),
            code: None,
            body: "oops".to_owned(),
        },
    ]
}

#[test]
fn given_every_case_when_round_tripped_then_identical() {
    for error in every_case() {
        let json = serde_json::to_value(&error).unwrap();
        assert_eq!(serde_json::from_value::<ClientError>(json).unwrap(), error);
    }
}

#[test]
fn given_every_case_when_asked_then_only_the_five_transient_ones_say_a_retry_could_help() {
    let retryable = [
        ClientError::RateLimited { retry_after: None },
        ClientError::Unavailable { retry_after: None },
        ClientError::ProviderFailed {
            status: None,
            code: None,
            body: String::new(),
        },
        ClientError::Transport,
        ClientError::StreamTruncated,
    ];
    for error in every_case() {
        let expected = retryable
            .iter()
            .any(|case| std::mem::discriminant(case) == std::mem::discriminant(&error));
        assert_eq!(error.is_retryable(), expected, "{error}");
    }
}

#[test]
fn given_a_quota_error_when_asked_then_waiting_is_not_advised() {
    assert!(!ClientError::QuotaExceeded.is_retryable());
}

#[test]
fn given_a_case_unknown_to_this_version_when_loaded_then_refused() {
    let stored = serde_json::json!({ "type": "teapot" });
    assert!(serde_json::from_value::<ClientError>(stored).is_err());
}

#[test]
fn given_a_limit_kind_unknown_to_this_version_when_loaded_then_refused() {
    let stored = serde_json::json!({
        "type": "limit_exceeded", "kind": "audio_length", "description": "x"
    });
    assert!(serde_json::from_value::<ClientError>(stored).is_err());
}

#[test]
fn given_a_reasoning_error_naming_a_level_unknown_to_this_version_when_loaded_then_refused() {
    let stored = serde_json::json!({
        "type": "reasoning_not_supported", "level": "ultra", "reason": "x"
    });
    assert!(serde_json::from_value::<ClientError>(stored).is_err());
}

#[test]
fn given_a_stored_error_with_an_unknown_field_when_loaded_then_refused() {
    let stored = serde_json::json!({
        "type": "rate_limited", "retry_after": { "secs": 1, "nanos": 0 }, "host": "example.org"
    });
    assert!(serde_json::from_value::<ClientError>(stored).is_err());
}

#[test]
fn given_the_absent_optional_items_when_stored_then_they_are_left_out() {
    let json = serde_json::to_value(ClientError::RateLimited { retry_after: None }).unwrap();
    assert_eq!(json, serde_json::json!({ "type": "rate_limited" }));
}

#[test]
fn given_a_provider_error_when_displayed_then_status_and_body_show_unchanged() {
    let error = ClientError::ProviderRejected {
        status: Some(418),
        code: None,
        body: "body as received".to_owned(),
    };
    let shown = error.to_string();
    assert!(shown.contains("418") && shown.contains("body as received"));
}

#[test]
fn given_an_extra_field_when_loaded_then_refusal_covers_structures_and_struct_cases_not_unit_cases()
{
    let extra_on_struct_case = serde_json::json!({
        "type": "rate_limited", "host": "example.org"
    });
    assert!(serde_json::from_value::<ClientError>(extra_on_struct_case).is_err());

    let extra_on_stored_structure = serde_json::json!({
        "total_duration": { "secs": 1, "nanos": 0 }, "host": "example.org"
    });
    assert!(serde_json::from_value::<crate::CallReport>(extra_on_stored_structure).is_err());

    let extra_on_unit_case = serde_json::json!({ "type": "transport", "host": "example.org" });
    assert_eq!(
        serde_json::from_value::<ClientError>(extra_on_unit_case).unwrap(),
        ClientError::Transport
    );
    let extra_on_plain = serde_json::json!({ "type": "plain", "host": "example.org" });
    assert_eq!(
        serde_json::from_value::<crate::OutputMode>(extra_on_plain).unwrap(),
        crate::OutputMode::Plain
    );
}
