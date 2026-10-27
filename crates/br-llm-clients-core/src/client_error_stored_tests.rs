use std::time::Duration;

use br_llm_messages::{MediaType, ResponseEnding, ToolName};
use serde_json::{Value, json};

use crate::client_error::{ClientError, LimitKind};
use crate::reasoning::ReasoningLevel;

fn spelled() -> Vec<(ClientError, Value)> {
    vec![
        (
            ClientError::ModelNotFound,
            json!({"type": "model_not_found"}),
        ),
        (
            ClientError::ModelNotAccessible,
            json!({"type": "model_not_accessible"}),
        ),
        (
            ClientError::Authentication,
            json!({"type": "authentication"}),
        ),
        (
            ClientError::PermissionDenied,
            json!({"type": "permission_denied"}),
        ),
        (
            ClientError::ContextTooLong { limit: Some(9) },
            json!({"type": "context_too_long", "limit": 9}),
        ),
        (
            ClientError::ContentBlocked,
            json!({"type": "content_blocked"}),
        ),
        (
            ClientError::ReplyUnusable {
                ending: ResponseEnding::output_exhaustion(),
            },
            json!({"type": "reply_unusable", "ending": {"type": "output_exhaustion"}}),
        ),
        (
            ClientError::ReplyUnusable {
                ending: ResponseEnding::unclassified("odd").unwrap(),
            },
            json!({"type": "reply_unusable", "ending": {"type": "unclassified", "reason": "odd"}}),
        ),
        (
            ClientError::InvalidMessages {
                reason: "r".to_owned(),
            },
            json!({"type": "invalid_messages", "reason": "r"}),
        ),
        (
            ClientError::QuotaExceeded,
            json!({"type": "quota_exceeded"}),
        ),
        (
            ClientError::RateLimited {
                retry_after: Some(Duration::new(3, 7)),
            },
            json!({"type": "rate_limited", "retry_after": {"secs": 3, "nanos": 7}}),
        ),
        (
            ClientError::Unavailable { retry_after: None },
            json!({"type": "unavailable"}),
        ),
        (
            ClientError::StreamTruncated,
            json!({"type": "stream_truncated"}),
        ),
        (ClientError::Transport, json!({"type": "transport"})),
        (
            ClientError::ReasoningNotSupported {
                level: ReasoningLevel::Extra,
                reason: "r".to_owned(),
            },
            json!({"type": "reasoning_not_supported", "level": "extra", "reason": "r"}),
        ),
        (
            ClientError::StructuredOutputNotSupported,
            json!({"type": "structured_output_not_supported"}),
        ),
        (
            ClientError::StructuredOutputWithToolsNotSupported,
            json!({"type": "structured_output_with_tools_not_supported"}),
        ),
        (
            ClientError::ToolsNotSupported,
            json!({"type": "tools_not_supported"}),
        ),
        (
            ClientError::CacheNotSupported,
            json!({"type": "cache_not_supported"}),
        ),
        (
            ClientError::ImageNotSupported,
            json!({"type": "image_not_supported"}),
        ),
        (
            ClientError::ImageMediaTypeNotSupported {
                media_type: MediaType::Png,
            },
            json!({"type": "image_media_type_not_supported", "media_type": "image/png"}),
        ),
        (
            ClientError::LimitExceeded {
                kind: LimitKind::SchemaDepth,
                description: "d".to_owned(),
            },
            json!({"type": "limit_exceeded", "kind": "schema_depth", "description": "d"}),
        ),
        (
            ClientError::MaxOutputTokensTooLarge { limit: None },
            json!({"type": "max_output_tokens_too_large"}),
        ),
        (
            ClientError::ToolSchemaRejected {
                tool: Some(ToolName::new("search").unwrap()),
                reason: "r".to_owned(),
            },
            json!({"type": "tool_schema_rejected", "tool": "search", "reason": "r"}),
        ),
        (
            ClientError::ToolSchemaRejected {
                tool: None,
                reason: "r".to_owned(),
            },
            json!({"type": "tool_schema_rejected", "reason": "r"}),
        ),
        (
            ClientError::OutputShapeRejected {
                reason: "r".to_owned(),
            },
            json!({"type": "output_shape_rejected", "reason": "r"}),
        ),
        (
            ClientError::MalformedResponse {
                detail: "d".to_owned(),
            },
            json!({"type": "malformed_response", "detail": "d"}),
        ),
        (
            ClientError::ProviderRejected {
                status: Some(422),
                code: Some("c".to_owned()),
                body: "b".to_owned(),
            },
            json!({"type": "provider_rejected", "status": 422, "code": "c", "body": "b"}),
        ),
        (
            ClientError::ProviderFailed {
                status: None,
                code: None,
                body: "b".to_owned(),
            },
            json!({"type": "provider_failed", "body": "b"}),
        ),
    ]
}

#[test]
fn given_every_case_when_stored_then_the_type_is_snake_case_and_the_fields_have_their_spelling() {
    for (error, expected) in spelled() {
        assert_eq!(serde_json::to_value(&error).unwrap(), expected, "{error}");
    }
}

#[test]
fn given_every_spelled_case_when_loaded_then_the_error_comes_back() {
    for (error, stored) in spelled() {
        assert_eq!(
            serde_json::from_value::<ClientError>(stored).unwrap(),
            error
        );
    }
}

#[test]
fn given_a_stored_case_missing_a_field_when_loaded_then_only_the_optional_items_may_be_absent() {
    const OPTIONAL: [&str; 5] = ["limit", "retry_after", "tool", "status", "code"];
    for (error, stored) in spelled() {
        for key in stored
            .as_object()
            .unwrap()
            .keys()
            .filter(|key| *key != "type")
        {
            let mut without = stored.clone();
            without.as_object_mut().unwrap().remove(key);
            let loaded = serde_json::from_value::<ClientError>(without);
            assert_eq!(
                loaded.is_ok(),
                OPTIONAL.contains(&key.as_str()),
                "{error}: {key}"
            );
        }
    }
}

#[test]
fn given_a_media_type_or_ending_this_version_does_not_know_when_loaded_then_refused() {
    let media = json!({"type": "image_media_type_not_supported", "media_type": "image/bmp"});
    assert!(serde_json::from_value::<ClientError>(media).is_err());
    let ending = json!({"type": "reply_unusable", "ending": {"type": "exploded"}});
    assert!(serde_json::from_value::<ClientError>(ending).is_err());
    let ending_key = json!({"type": "reply_unusable", "ending": {"type": "finished", "x": 1}});
    assert!(serde_json::from_value::<ClientError>(ending_key).is_err());
}

#[test]
fn given_a_null_for_an_optional_item_when_loaded_then_refused() {
    let stored = json!({"type": "rate_limited", "retry_after": null});
    assert!(serde_json::from_value::<ClientError>(stored).is_err());
}

#[test]
fn given_types_without_display_when_the_error_is_shown_then_the_text_uses_their_string_form() {
    let media = ClientError::ImageMediaTypeNotSupported {
        media_type: MediaType::Gif,
    };
    assert_eq!(
        media.to_string(),
        "the model does not take images of type image/gif"
    );
    let unusable = ClientError::ReplyUnusable {
        ending: ResponseEnding::output_exhaustion(),
    };
    assert!(unusable.to_string().contains("OutputExhaustion"));
    let duplicate = crate::BuildError::DuplicateToolName {
        name: ToolName::new("search").unwrap(),
    };
    assert_eq!(duplicate.to_string(), "two tools share the name search");
}
