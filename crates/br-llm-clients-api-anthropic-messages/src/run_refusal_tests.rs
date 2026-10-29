use std::time::Duration;

use br_llm_clients_core::{CacheRequirement, ClientError, LimitKind, ReasoningLevel};
use br_llm_messages::ResponseEnding;

use crate::test_call::*;
use crate::test_fixtures::dialect_with;
use crate::test_server::http;

type Row = (
    &'static str,
    Vec<(&'static str, &'static str)>,
    String,
    ClientError,
);

fn error_body(code: &str, message: &str) -> String {
    serde_json::json!({
        "type": "error",
        "error": {"type": code, "message": message},
        "request_id": "req_1"
    })
    .to_string()
}

const SPEND_CAP_BODY: &str = r#"{"type":"error","error":{"type":"rate_limit_error","message":"You have reached your API usage limits.","details":{"error_code":"enforced_spend_limit_reached"}},"request_id":"req_1"}"#;

#[tokio::test]
async fn given_the_documented_statuses_when_run_then_the_errors_follow_the_tables() {
    let overloaded = error_body("overloaded_error", "Overloaded");
    let rows: Vec<Row> = vec![
        (
            "401 Unauthorized",
            vec![],
            error_body("authentication_error", "bad key"),
            ClientError::Authentication,
        ),
        (
            "403 Forbidden",
            vec![],
            error_body("permission_error", "no"),
            ClientError::PermissionDenied,
        ),
        (
            "402 Payment Required",
            vec![],
            error_body("billing_error", "pay"),
            ClientError::QuotaExceeded,
        ),
        (
            "402 Payment Required",
            vec![],
            "pay up".to_owned(),
            ClientError::QuotaExceeded,
        ),
        (
            "429 Too Many Requests",
            vec![("retry-after", "7")],
            error_body("rate_limit_error", "slow down"),
            ClientError::RateLimited {
                retry_after: Some(Duration::from_secs(7)),
            },
        ),
        (
            "429 Too Many Requests",
            vec![],
            SPEND_CAP_BODY.to_owned(),
            ClientError::QuotaExceeded,
        ),
        (
            "529 Overloaded",
            vec![("retry-after", "3")],
            overloaded.clone(),
            ClientError::Unavailable {
                retry_after: Some(Duration::from_secs(3)),
            },
        ),
        (
            "529 Overloaded",
            vec![],
            overloaded,
            ClientError::Unavailable { retry_after: None },
        ),
        (
            "400 Bad Request",
            vec![],
            error_body(
                "invalid_request_error",
                "prompt is too long: 9 tokens > 8 maximum",
            ),
            ClientError::ContextTooLong { limit: None },
        ),
        (
            "400 Bad Request",
            vec![],
            error_body(
                "invalid_request_error",
                "You have reached your specified workspace API usage limits. You will regain access on 2026-09-01",
            ),
            ClientError::QuotaExceeded,
        ),
    ];
    for (status, headers, body, expected) in rows {
        assert_eq!(
            call_refusal(status, &headers, &body).await,
            expected,
            "{status} {body}"
        );
    }
}

#[tokio::test]
async fn given_a_413_when_run_then_it_is_a_limit_of_kind_other_with_the_body() {
    let body = error_body(
        "request_too_large",
        "Request exceeds the maximum allowed number of bytes.",
    );
    for with_json in [true, false] {
        let text = if with_json {
            body.clone()
        } else {
            "too big".to_owned()
        };
        assert_eq!(
            call_refusal("413 Payload Too Large", &[], &text).await,
            ClientError::LimitExceeded {
                kind: LimitKind::Other,
                description: text
            }
        );
    }
}

#[tokio::test]
async fn given_a_not_found_when_run_then_it_is_a_rejection_never_model_not_found() {
    let body = error_body(
        "not_found_error",
        "The requested resource could not be found.",
    );
    assert_eq!(
        call_refusal("404 Not Found", &[], &body).await,
        ClientError::ProviderRejected {
            status: Some(404),
            code: Some("not_found_error".to_owned()),
            body,
        }
    );
}

#[tokio::test]
async fn given_a_400_about_max_tokens_when_run_then_it_is_a_rejection_never_a_max_tokens_error() {
    let body = error_body(
        "invalid_request_error",
        "max_tokens: 200000 > 64000, which is the maximum allowed number of output tokens for claude-test-model",
    );
    assert_eq!(
        call_refusal("400 Bad Request", &[], &body).await,
        ClientError::ProviderRejected {
            status: Some(400),
            code: Some("invalid_request_error".to_owned()),
            body,
        }
    );
}

#[tokio::test]
async fn given_a_server_error_when_run_then_it_is_a_failure_of_the_provider() {
    let body = error_body("api_error", "oops");
    assert_eq!(
        call_refusal("500 Internal Server Error", &[], &body).await,
        ClientError::ProviderFailed {
            status: Some(500),
            code: Some("api_error".to_owned()),
            body,
        }
    );
}

#[tokio::test]
async fn given_a_tool_id_the_api_refuses_when_run_then_it_comes_back_as_a_rejection() {
    let body = error_body(
        "invalid_request_error",
        "messages.1.content.0.tool_use.id: String should match pattern '^[a-zA-Z0-9_-]+$'",
    );
    let error = call_refusal("400 Bad Request", &[], &body).await;
    assert!(matches!(
        error,
        ClientError::ProviderRejected {
            status: Some(400),
            ..
        }
    ));
}

#[tokio::test]
async fn given_the_thinking_phrases_with_a_level_when_run_then_the_error_carries_the_level_and_message()
 {
    let messages = [
        "adaptive thinking is not supported on this model",
        "\"thinking.type.disabled\" is not supported for this model. Use \"thinking.type.adaptive\" and \"output_config.effort\" to control thinking behavior.",
    ];
    for message in messages {
        let body = error_body("invalid_request_error", message);
        let dialect = dialect_with(ReasoningLevel::High, CacheRequirement::NotRequired);
        let owned = body.clone();
        let called = call_with(dialect, &hello(), move |_| {
            http("400 Bad Request", &[], owned.as_bytes())
        })
        .await;
        assert_eq!(
            called.error(),
            &ClientError::ReasoningNotSupported {
                level: ReasoningLevel::High,
                reason: message.to_owned(),
            }
        );
    }
}

#[tokio::test]
async fn given_a_mid_stream_overloaded_error_when_run_then_it_is_unavailable_and_nothing_more_is_read()
 {
    let called = call_stream(&hello(), crate::test_streams::MID_STREAM_ERROR).await;
    assert_eq!(
        called.error(),
        &ClientError::Unavailable { retry_after: None }
    );
}

#[tokio::test]
async fn given_a_stream_with_no_event_names_when_run_then_the_type_of_the_data_decides() {
    let called = call_stream(&hello(), &without_event_names(crate::test_streams::BASIC)).await;
    assert_eq!(called.response().ending(), &ResponseEnding::finished());
    let called = call_stream(
        &hello(),
        &without_event_names(crate::test_streams::MID_STREAM_ERROR),
    )
    .await;
    assert_eq!(
        called.error(),
        &ClientError::Unavailable { retry_after: None }
    );
}

#[tokio::test]
async fn given_a_mid_stream_error_of_another_type_when_run_then_it_is_a_failure_of_the_provider() {
    let stream = crate::test_streams::MID_STREAM_ERROR.replace("overloaded_error", "api_error");
    let called = call_stream(&hello(), &stream).await;
    assert!(matches!(
        called.error(),
        ClientError::ProviderFailed { status: None, code: Some(code), .. } if code == "api_error"
    ));
}

#[tokio::test]
async fn given_mid_stream_errors_with_the_table_codes_when_run_then_each_follows_the_code_table() {
    let rows = [
        (
            "rate_limit_error",
            ClientError::RateLimited { retry_after: None },
        ),
        ("billing_error", ClientError::QuotaExceeded),
        ("authentication_error", ClientError::Authentication),
        ("permission_error", ClientError::PermissionDenied),
    ];
    for (code, expected) in rows {
        let stream = crate::test_streams::MID_STREAM_ERROR.replace("overloaded_error", code);
        let called = call_stream(&hello(), &stream).await;
        assert_eq!(called.error(), &expected, "{code}");
    }
}
