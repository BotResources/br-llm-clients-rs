use std::time::Duration;

use br_llm_clients_core::ClientError;
use serde_json::json;

use crate::run_support::{call_refused, call_stream};
use crate::test_support::{chunk, content_chunk, finish_chunk};

fn error_body(code: Option<&str>, kind: &str) -> String {
    json!({"error": {"message": "the message", "type": kind, "param": null, "code": code}})
        .to_string()
}

#[tokio::test]
async fn given_a_429_with_insufficient_quota_when_called_then_the_quota_is_exceeded() {
    let body = error_body(Some("insufficient_quota"), "insufficient_quota");
    let called = call_refused("429 Too Many Requests", &[("retry-after", "30")], &body).await;
    assert_eq!(called.error(), &ClientError::QuotaExceeded);
}

#[tokio::test]
async fn given_a_429_with_insufficient_quota_only_in_the_type_when_called_then_it_is_rate_limited()
{
    let body = error_body(Some("rate_limit_exceeded"), "insufficient_quota");
    let called = call_refused("429 Too Many Requests", &[], &body).await;
    assert_eq!(
        called.error(),
        &ClientError::RateLimited { retry_after: None }
    );
}

#[tokio::test]
async fn given_a_429_for_the_rate_when_called_then_it_is_rate_limited_with_the_delay() {
    let body = error_body(Some("rate_limit_exceeded"), "requests");
    let called = call_refused("429 Too Many Requests", &[("retry-after", "3")], &body).await;
    assert_eq!(
        called.error(),
        &ClientError::RateLimited {
            retry_after: Some(Duration::from_secs(3))
        }
    );
}

#[tokio::test]
async fn given_a_429_whose_type_is_insufficient_quota_and_no_code_when_called_then_the_type_is_the_code()
 {
    let body = error_body(None, "insufficient_quota");
    let called = call_refused("429 Too Many Requests", &[], &body).await;
    assert_eq!(called.error(), &ClientError::QuotaExceeded);
}

#[tokio::test]
async fn given_a_400_when_called_then_the_rejection_carries_the_status_the_code_and_the_body() {
    let body = error_body(Some("model_not_found"), "invalid_request_error");
    let called = call_refused("400 Bad Request", &[], &body).await;
    match called.error() {
        ClientError::ProviderRejected {
            status,
            code,
            body: text,
        } => {
            assert_eq!(*status, Some(400));
            assert_eq!(code.as_deref(), Some("model_not_found"));
            assert!(text.contains("the message"));
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[tokio::test]
async fn given_an_over_large_token_count_when_called_then_it_is_a_provider_rejection_not_a_specific_case()
 {
    let body = json!({"error": {
        "message": "max_tokens is too large: 9999999. This model supports at most 16384 completion tokens",
        "type": "invalid_request_error", "param": "max_tokens", "code": null
    }})
    .to_string();
    let called = call_refused("400 Bad Request", &[], &body).await;
    match called.error() {
        ClientError::ProviderRejected { status, code, .. } => {
            assert_eq!(*status, Some(400));
            assert_eq!(code.as_deref(), Some("invalid_request_error"));
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[tokio::test]
async fn given_the_generic_statuses_when_called_then_the_generic_table_applies() {
    let body = error_body(Some("invalid_api_key"), "invalid_request_error");
    assert_eq!(
        call_refused("401 Unauthorized", &[], &body).await.error(),
        &ClientError::Authentication
    );
    assert_eq!(
        call_refused("403 Forbidden", &[], &body).await.error(),
        &ClientError::PermissionDenied
    );
    assert_eq!(
        call_refused("503 Service Unavailable", &[("retry-after", "2")], &body)
            .await
            .error(),
        &ClientError::Unavailable {
            retry_after: Some(Duration::from_secs(2))
        }
    );
}

#[tokio::test]
async fn given_a_500_with_a_body_that_is_not_json_when_called_then_the_failure_has_no_code() {
    let called = call_refused(
        "500 Internal Server Error",
        &[],
        "<html>upstream error</html>",
    )
    .await;
    match called.error() {
        ClientError::ProviderFailed { status, code, body } => {
            assert_eq!(*status, Some(500));
            assert_eq!(*code, None);
            assert!(body.contains("upstream error"));
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[tokio::test]
async fn given_an_error_frame_inside_the_stream_when_called_then_the_failure_is_a_provider_failure()
{
    let frame = json!({"error": {"message": "The server had an error", "type": "server_error", "code": null}}).to_string();
    let called = call_stream(&[content_chunk("partial"), frame]).await;
    match called.error() {
        ClientError::ProviderFailed { status, code, body } => {
            assert_eq!(*status, None);
            assert_eq!(code.as_deref(), Some("server_error"));
            assert!(body.contains("The server had an error"));
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[tokio::test]
async fn given_an_insufficient_quota_frame_inside_the_stream_when_called_then_the_quota_is_exceeded()
 {
    let frame = json!({"error": {"message": "quota", "type": "insufficient_quota", "code": "insufficient_quota"}}).to_string();
    let called = call_stream(&[frame]).await;
    assert_eq!(called.error(), &ClientError::QuotaExceeded);
}

#[tokio::test]
async fn given_the_finish_reason_error_when_called_then_the_failure_is_a_provider_failure_without_status()
 {
    let called = call_stream(&[content_chunk("partial"), finish_chunk("error")]).await;
    assert!(matches!(
        called.error(),
        ClientError::ProviderFailed {
            status: None,
            code: None,
            ..
        }
    ));
}

#[tokio::test]
async fn given_an_error_frame_after_a_finish_when_called_then_the_error_still_fails_the_call() {
    let frame = json!({"error": {"message": "late", "type": "server_error"}}).to_string();
    let called = call_stream(&[content_chunk("done"), finish_chunk("stop"), frame]).await;
    assert!(matches!(called.error(), ClientError::ProviderFailed { .. }));
}

#[tokio::test]
async fn given_a_payload_that_is_not_json_when_called_then_the_reply_is_malformed() {
    let called = call_stream(&["this is not json".to_owned()]).await;
    assert!(matches!(
        called.error(),
        ClientError::MalformedResponse { .. }
    ));
}

#[tokio::test]
async fn given_a_chunk_with_two_choices_when_called_then_the_reply_is_malformed() {
    let two =
        json!({"choices": [{"index": 0, "delta": {}}, {"index": 1, "delta": {}}]}).to_string();
    let called = call_stream(&[two]).await;
    assert!(matches!(
        called.error(),
        ClientError::MalformedResponse { .. }
    ));
}

#[tokio::test]
async fn given_audio_in_a_delta_when_called_then_the_reply_is_malformed() {
    let audio = chunk(json!({"audio": {"data": "AAAA"}}), None);
    let called = call_stream(&[audio]).await;
    assert!(matches!(
        called.error(),
        ClientError::MalformedResponse { .. }
    ));
}

#[tokio::test]
async fn given_a_second_finish_when_called_then_the_reply_is_malformed() {
    let called = call_stream(&[
        content_chunk("a"),
        finish_chunk("stop"),
        finish_chunk("stop"),
    ])
    .await;
    assert!(matches!(
        called.error(),
        ClientError::MalformedResponse { .. }
    ));
}

#[tokio::test]
async fn given_a_finish_with_no_sentinel_when_called_then_the_stream_is_truncated() {
    let bytes = crate::run_support::sse(&[content_chunk("a"), finish_chunk("stop")]);
    let called = crate::run_support::call_with(
        crate::test_support::dialect(),
        &crate::test_support::hello(),
        move |_| crate::run_support::ok_stream(&bytes),
    )
    .await;
    assert_eq!(called.error(), &ClientError::StreamTruncated);
}

#[tokio::test]
async fn given_a_sentinel_with_no_finish_when_called_then_the_stream_is_truncated() {
    let called = call_stream(&[content_chunk("a")]).await;
    assert_eq!(called.error(), &ClientError::StreamTruncated);
}

#[tokio::test]
async fn given_an_empty_body_when_called_then_the_stream_is_truncated() {
    let called = crate::run_support::call_with(
        crate::test_support::dialect(),
        &crate::test_support::hello(),
        |_| crate::run_support::ok_stream(b""),
    )
    .await;
    assert_eq!(called.error(), &ClientError::StreamTruncated);
}
