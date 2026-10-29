use br_llm_clients_core::ClientError;
use serde_json::json;

use crate::test_events::{created, error_event, failed, message_added, text_delta, usage_object};
use crate::test_requests::hello;
use crate::test_run::{Called, call, call_stream, ok_wire};

fn failure_of(called: Called) -> ClientError {
    called.outcome.into_parts().1.expect_err("a failure")
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
    assert_eq!(
        report.usage().and_then(|usage| usage.input_tokens()),
        Some(7)
    );
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
