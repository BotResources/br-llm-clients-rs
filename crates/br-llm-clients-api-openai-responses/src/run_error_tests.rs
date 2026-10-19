use std::time::Duration;

use br_llm_clients_core::ClientError;
use serde_json::json;

use crate::test_events::{created, error_event, failed, message_added, text_delta, usage_object};
use crate::test_requests::hello;
use crate::test_run::{Called, call, call_stream, ok_wire};
use crate::test_server::http;

async fn refused(status: &str, headers: &[(&str, &str)], body: &str) -> Called {
    call(&hello(), http(status, headers, body.as_bytes())).await
}

fn failure_of(called: Called) -> ClientError {
    called.outcome.into_parts().1.expect_err("a failure")
}

fn body(code: Option<&str>, kind: &str) -> String {
    json!({ "error": { "message": "provider message", "type": kind, "param": null, "code": code } })
        .to_string()
}

#[tokio::test]
async fn given_a_429_with_a_rate_limit_code_and_retry_after_when_run_then_it_is_rate_limited_with_the_delay()
 {
    let called = refused(
        "429 Too Many Requests",
        &[("retry-after", "2")],
        &body(Some("rate_limit_exceeded"), "rate_limit_error"),
    )
    .await;
    assert_eq!(
        failure_of(called),
        ClientError::RateLimited {
            retry_after: Some(Duration::from_secs(2))
        }
    );
}

#[tokio::test]
async fn given_a_429_with_the_quota_type_when_run_then_it_is_quota_exceeded_through_the_type_fallback()
 {
    let called = refused(
        "429 Too Many Requests",
        &[("retry-after", "2")],
        &body(None, "insufficient_quota"),
    )
    .await;
    assert_eq!(failure_of(called), ClientError::QuotaExceeded);
}

#[tokio::test]
async fn given_a_429_with_a_quota_code_when_run_then_it_is_quota_exceeded_and_not_rate_limited() {
    let called = refused(
        "429 Too Many Requests",
        &[],
        &body(Some("insufficient_quota"), "insufficient_quota"),
    )
    .await;
    assert_eq!(failure_of(called), ClientError::QuotaExceeded);
}

#[tokio::test]
async fn given_a_slow_down_429_without_retry_after_when_run_then_it_is_rate_limited_without_delay()
{
    let called = refused(
        "429 Too Many Requests",
        &[],
        &body(Some("slow_down"), "rate_limit_error"),
    )
    .await;
    assert_eq!(
        failure_of(called),
        ClientError::RateLimited { retry_after: None }
    );
}

#[tokio::test]
async fn given_a_503_overloaded_with_a_delay_when_run_then_it_is_unavailable_with_the_delay() {
    let called = refused(
        "503 Service Unavailable",
        &[("retry-after", "5")],
        &body(Some("server_is_overloaded"), "service_unavailable_error"),
    )
    .await;
    assert_eq!(
        failure_of(called),
        ClientError::Unavailable {
            retry_after: Some(Duration::from_secs(5))
        }
    );
}

#[tokio::test]
async fn given_a_plain_503_when_run_then_the_generic_table_makes_it_unavailable() {
    let called = refused("503 Service Unavailable", &[], "upstream down").await;
    assert!(matches!(
        failure_of(called),
        ClientError::Unavailable { .. }
    ));
}

#[tokio::test]
async fn given_a_policy_code_over_http_when_run_then_the_content_is_blocked() {
    let called = refused(
        "400 Bad Request",
        &[],
        &body(Some("bio_policy"), "invalid_request_error"),
    )
    .await;
    assert_eq!(failure_of(called), ClientError::ContentBlocked);
}

#[tokio::test]
async fn given_the_generic_statuses_when_run_then_the_generic_table_decides() {
    let invalid = body(Some("invalid_api_key"), "invalid_request_error");
    assert_eq!(
        failure_of(refused("401 Unauthorized", &[], &invalid).await),
        ClientError::Authentication
    );
    assert_eq!(
        failure_of(refused("403 Forbidden", &[], &invalid).await),
        ClientError::PermissionDenied
    );
    let rejected = failure_of(
        refused(
            "404 Not Found",
            &[],
            &body(Some("model_not_found"), "invalid_request_error"),
        )
        .await,
    );
    assert!(matches!(
        rejected,
        ClientError::ProviderRejected { status: Some(404), code: Some(ref code), .. } if code == "model_not_found"
    ));
    let failed = failure_of(
        refused(
            "500 Internal Server Error",
            &[],
            &body(Some("server_error"), "server_error"),
        )
        .await,
    );
    assert!(matches!(
        failed,
        ClientError::ProviderFailed { status: Some(500), code: Some(ref code), .. } if code == "server_error"
    ));
}

#[tokio::test]
async fn given_a_400_for_a_too_long_input_when_run_then_it_is_a_provider_rejection_not_a_context_case()
 {
    let called = refused(
        "400 Bad Request",
        &[],
        &body(Some("context_length_exceeded"), "invalid_request_error"),
    )
    .await;
    assert!(matches!(
        failure_of(called),
        ClientError::ProviderRejected {
            status: Some(400),
            ..
        }
    ));
}

#[tokio::test]
async fn given_a_400_with_a_request_blaming_code_and_a_status_when_run_then_the_generic_table_carries_status_and_code()
 {
    let called = refused(
        "400 Bad Request",
        &[],
        &body(Some("invalid_prompt"), "invalid_request_error"),
    )
    .await;
    assert!(matches!(
        failure_of(called),
        ClientError::ProviderRejected { status: Some(400), code: Some(ref code), .. } if code == "invalid_prompt"
    ));
}

#[tokio::test]
async fn given_a_refusal_body_that_is_not_json_when_run_then_the_generic_table_gives_no_code() {
    let called = refused("502 Bad Gateway", &[], "<html>Bad gateway</html>").await;
    assert!(matches!(
        failure_of(called),
        ClientError::ProviderFailed {
            status: Some(502),
            code: None,
            ..
        }
    ));
}

#[tokio::test]
async fn given_a_failed_response_with_a_rate_limit_code_when_run_then_it_is_rate_limited() {
    let events = vec![
        created("m"),
        failed("rate_limit_exceeded", "Rate limit reached"),
    ];
    assert_eq!(
        failure_of(call_stream(&hello(), &events).await),
        ClientError::RateLimited { retry_after: None }
    );
}

#[tokio::test]
async fn given_a_failed_response_with_a_server_error_when_run_then_it_is_a_provider_failure_with_code_and_envelope()
 {
    let events = vec![created("m"), failed("server_error", "The model failed")];
    let failure = failure_of(call_stream(&hello(), &events).await);
    match failure {
        ClientError::ProviderFailed { status, code, body } => {
            assert_eq!(status, None);
            assert_eq!(code.as_deref(), Some("server_error"));
            assert!(body.contains("The model failed"));
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn given_a_failed_response_blaming_the_request_when_run_then_it_is_a_provider_rejection_without_status()
 {
    for code in [
        "invalid_prompt",
        "invalid_image_url",
        "data_residency_mismatch",
    ] {
        let events = vec![created("m"), failed(code, "bad request")];
        let failure = failure_of(call_stream(&hello(), &events).await);
        assert!(
            matches!(
                &failure,
                ClientError::ProviderRejected { status: None, code: Some(found), .. } if found == code
            ),
            "{code}: {failure:?}"
        );
    }
}

#[tokio::test]
async fn given_a_failed_response_with_a_policy_code_when_run_then_the_content_is_blocked() {
    for code in [
        "bio_policy",
        "misalignment_policy_violation",
        "image_content_policy_violation",
    ] {
        let events = vec![created("m"), failed(code, "blocked")];
        assert_eq!(
            failure_of(call_stream(&hello(), &events).await),
            ClientError::ContentBlocked,
            "{code}"
        );
    }
}

#[tokio::test]
async fn given_a_failed_response_with_usage_when_run_then_the_report_keeps_the_usage_and_the_model()
{
    let mut event = failed("server_error", "x");
    event["response"]["usage"] = usage_object(7, 8);
    let called = call_stream(&hello(), &[created("m"), event]).await;
    let (report, result) = called.outcome.into_parts();
    assert!(result.is_err());
    assert_eq!(report.usage().map(|usage| usage.input_tokens), Some(7));
    assert_eq!(
        report.model().map(|model| model.as_str()),
        Some("gpt-5.1-2026-01-01")
    );
}

#[tokio::test]
async fn given_a_top_level_error_event_when_run_then_the_code_decides_the_case() {
    let overloaded = vec![error_event(
        Some("server_is_overloaded"),
        "overloaded",
        None,
    )];
    assert_eq!(
        failure_of(call_stream(&hello(), &overloaded).await),
        ClientError::Unavailable { retry_after: None }
    );
    let plain = vec![error_event(None, "Something went wrong", None)];
    assert!(matches!(
        failure_of(call_stream(&hello(), &plain).await),
        ClientError::ProviderFailed {
            status: None,
            code: None,
            ..
        }
    ));
}

#[tokio::test]
async fn given_an_error_event_after_content_when_run_then_the_error_wins_over_the_partial_reply() {
    let events = vec![
        created("m"),
        message_added(0),
        text_delta(0, 0, "partial"),
        error_event(Some("server_error"), "boom", None),
    ];
    assert!(matches!(
        failure_of(call_stream(&hello(), &events).await),
        ClientError::ProviderFailed { .. }
    ));
}

#[tokio::test]
async fn given_an_unknown_output_item_in_the_stream_when_run_then_the_reply_is_malformed() {
    let events = vec![
        created("m"),
        json!({ "type": "response.output_item.added", "output_index": 0, "item": { "id": "ws_1", "type": "web_search_call", "status": "in_progress" } }),
    ];
    assert!(matches!(
        failure_of(call_stream(&hello(), &events).await),
        ClientError::MalformedResponse { .. }
    ));
}

#[tokio::test]
async fn given_data_that_is_not_json_in_a_frame_when_run_then_the_reply_is_malformed() {
    let wire = ok_wire(b"event: response.created\ndata: {not json\n\n");
    let failure = failure_of(call(&hello(), wire).await);
    assert!(matches!(failure, ClientError::MalformedResponse { .. }));
}

#[tokio::test]
async fn given_a_payload_error_when_run_then_the_malformed_detail_never_quotes_the_payload() {
    let secret = "SENTINEL-prompt-3b9e";
    let wire = ok_wire(format!("data: {{\"type\":\"response.output_text.delta\",\"output_index\":0,\"content_index\":0,\"delta\":\"{secret}\"}}\n\n").as_bytes());
    let failure = failure_of(call(&hello(), wire).await);
    match failure {
        ClientError::MalformedResponse { detail } => assert!(!detail.contains(secret)),
        other => panic!("{other:?}"),
    }
}
