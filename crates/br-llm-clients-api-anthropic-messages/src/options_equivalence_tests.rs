use br_llm_clients_core::{ClientError, ParameterKind};
use serde_json::json;

use crate::test_call::*;
use crate::test_fixtures::*;
use crate::test_server::http;
use crate::test_streams::{BASIC, MID_STREAM_ERROR, THINKING, TOOL_USE};
use crate::test_streams_constructed::{REDACTED_THINKING, REFUSAL};
use crate::{MessagesOptions, ModelPlacement, VersionPlacement};

fn every_option_set() -> Vec<MessagesOptions> {
    let version = VersionPlacement::Body("vertex-2023-10-16".to_owned());
    let extra = json!({"metadata": {"user_id": "u1"}, "temperature": 0.5})
        .as_object()
        .cloned()
        .expect("object");
    vec![
        MessagesOptions::new(),
        MessagesOptions::new().with_model_placement(ModelPlacement::Omitted),
        MessagesOptions::new().with_version_placement(version.clone()),
        MessagesOptions::new()
            .with_extra_body(extra.clone())
            .expect("accepted"),
        MessagesOptions::new()
            .with_model_placement(ModelPlacement::Omitted)
            .with_version_placement(version)
            .with_extra_body(extra)
            .expect("accepted"),
    ]
}

fn refusable() -> Vec<br_llm_clients_core::ClientRequest> {
    let weather = tool("weather", "", vec![]);
    vec![
        request(vec![user("a"), assistant_text("b")]),
        request(vec![user_blocks(vec![])]),
        request(vec![
            user("a"),
            assistant(vec![tool_call("t1", "weather", json!([1]))]),
            user_blocks(vec![tool_result("t1", "weather", &["r"], false)]),
        ]),
        request(vec![
            user("a"),
            assistant(vec![thinking("hm", None)]),
            relay(&["x"]),
        ]),
        request_with(
            None,
            vec![user("hello")],
            vec![weather],
            structured(None, vec![param("v", "", true, ParameterKind::String)]),
        ),
    ]
}

#[test]
fn given_a_request_refused_by_default_when_encoded_under_every_option_then_the_same_case_is_given()
{
    use br_llm_clients_common_call::Dialect;
    for shape in refusable() {
        let default = refusal_of(&dialect(), &shape);
        for options in every_option_set() {
            let refused = dialect_options(options)
                .encode(&shape)
                .expect_err("refused");
            assert_eq!(refused, default);
        }
    }
    let kinds: Vec<_> = refusable()
        .iter()
        .map(|shape| refusal_of(&dialect(), shape))
        .collect();
    assert!(matches!(
        kinds[4],
        ClientError::StructuredOutputWithToolsNotSupported
    ));
    assert!(
        kinds[..4]
            .iter()
            .all(|kind| matches!(kind, ClientError::InvalidMessages { .. }))
    );
}

async fn run_stream(
    options: MessagesOptions,
    request: &br_llm_clients_core::ClientRequest,
    stream: &str,
) -> Called {
    let bytes = stream.as_bytes().to_vec();
    call_with(dialect_options(options), request, move |_| {
        http("200 OK", &[("content-type", "text/event-stream")], &bytes)
    })
    .await
}

#[tokio::test]
async fn given_the_recorded_streams_when_run_under_every_option_then_the_events_and_outcome_are_the_same()
 {
    let request = hello();
    for stream in [
        BASIC,
        TOOL_USE,
        THINKING,
        REDACTED_THINKING,
        REFUSAL,
        MID_STREAM_ERROR,
    ] {
        let default = run_stream(MessagesOptions::new(), &request, stream).await;
        for options in every_option_set() {
            let other = run_stream(options, &request, stream).await;
            assert_eq!(other.events, default.events);
            assert_eq!(other.outcome.result(), default.outcome.result());
        }
    }
}

#[tokio::test]
async fn given_the_recorded_error_when_run_under_every_option_then_the_client_error_is_the_same() {
    let body = json!({
        "type": "error",
        "error": {"type": "overloaded_error", "message": "Overloaded"},
        "request_id": "req_1"
    })
    .to_string();
    let request = hello();
    for options in every_option_set() {
        let called = call_with(dialect_options(options), &request, {
            let body = body.clone();
            move |_| {
                http(
                    "529 Overloaded",
                    &[("request-id", "req_1")],
                    body.as_bytes(),
                )
            }
        })
        .await;
        assert_eq!(
            called.error(),
            &ClientError::Unavailable { retry_after: None }
        );
    }
}

#[tokio::test]
async fn given_any_headers_when_the_call_is_refused_then_the_dialect_names_no_answered_model() {
    let request = hello();
    for options in every_option_set() {
        let called = call_with(dialect_options(options), &request, move |_| {
            http(
                "500 Internal Server Error",
                &[
                    ("x-model", "gw-model"),
                    ("model", "gw-model"),
                    ("anthropic-model", "gw-model"),
                    ("x-litellm-model-id", "gw-model"),
                ],
                b"{}",
            )
        })
        .await;
        assert!(called.outcome.report().model().is_none());
    }
}
