use std::sync::Arc;

use br_llm_clients_common_call::Dialect;
use br_llm_clients_core::{ClientError, ClientRequest};
use br_llm_messages::{AssistantBlock, StopReason};
use serde_json::{Value, json};

use crate::test_call::*;
use crate::test_fixtures::*;
use crate::test_server::{TestServer, http, http_bytes};
use crate::test_streams::BASIC;

fn conversation() -> ClientRequest {
    request_with(
        Some("be brief"),
        vec![
            user("weather?"),
            assistant(vec![
                thinking("hm", Some("sig")),
                tool_call("t1", "weather", json!({"city": "Paris"})),
            ]),
            user_blocks(vec![tool_result("t1", "weather", &["sunny"], false)]),
        ],
        vec![tool("weather", "the weather", vec![])],
        plain_output(),
    )
}

#[tokio::test]
async fn given_a_request_when_run_then_the_server_gets_a_post_with_the_encoded_body_and_the_two_headers()
 {
    let request = conversation();
    let called = call_stream(&request, BASIC).await;
    assert!(called.outcome.result().is_ok());
    let [received] = called.received.as_slice() else {
        panic!("one request expected");
    };
    assert!(received.request_line.starts_with("POST /v1/messages "));
    assert_eq!(received.header_values("content-type"), ["application/json"]);
    assert_eq!(received.header_values("anthropic-version"), ["2023-06-01"]);
    assert_eq!(received.header_values("accept"), ["text/event-stream"]);
    assert!(received.header_values("anthropic-beta").is_empty());
    assert!(received.header_values("x-api-key").is_empty());
    let sent: Value = serde_json::from_slice(&received.body).expect("json");
    let expected: Value =
        serde_json::from_slice(plain_dialect().encode(&request).expect("encoded").body())
            .expect("json");
    assert_eq!(sent, expected);
    assert_eq!(sent["stream"], json!(true));
}

#[tokio::test]
async fn given_a_replayed_turn_when_run_then_the_thinking_signature_travels_on_the_wire() {
    let called = call_stream(&conversation(), BASIC).await;
    let sent: Value = serde_json::from_slice(&called.received[0].body).expect("json");
    assert_eq!(sent["messages"][1]["content"][0]["signature"], json!("sig"));
}

#[tokio::test]
async fn given_a_refused_encoding_when_run_then_nothing_is_sent_and_the_error_is_given() {
    let refused = request(vec![user("a"), assistant_text("b")]);
    let called = call_stream(&refused, BASIC).await;
    assert!(matches!(
        called.error(),
        ClientError::InvalidMessages { .. }
    ));
    assert!(called.received.is_empty());
    let with_tools = request_with(
        None,
        vec![user("a")],
        vec![tool("weather", "", vec![])],
        structured(None, vec![]),
    );
    let called = call_stream(&with_tools, BASIC).await;
    assert_eq!(
        called.error(),
        &ClientError::StructuredOutputWithToolsNotSupported
    );
    assert!(called.received.is_empty());
}

#[tokio::test]
async fn given_a_generated_tool_id_in_history_when_run_then_the_id_is_sent_as_it_is() {
    let request = request(vec![
        user("go"),
        assistant(vec![tool_call("brgen_abc123", "weather", json!({}))]),
        user_blocks(vec![tool_result("brgen_abc123", "weather", &["r"], false)]),
    ]);
    let called = call_stream(&request, BASIC).await;
    let sent: Value = serde_json::from_slice(&called.received[0].body).expect("json");
    assert_eq!(
        sent["messages"][1]["content"][0]["id"],
        json!("brgen_abc123")
    );
    assert_eq!(
        sent["messages"][2]["content"][0]["tool_use_id"],
        json!("brgen_abc123")
    );
}

async fn request_id_for(headers: Vec<(&'static str, Vec<u8>)>) -> Option<String> {
    let body = BASIC.as_bytes().to_vec();
    let called = call_with(plain_dialect(), &hello(), move |_| {
        let borrowed: Vec<(&str, &[u8])> = headers
            .iter()
            .map(|(name, value)| (*name, value.as_slice()))
            .collect();
        http_bytes("200 OK", &borrowed, &body)
    })
    .await;
    called.outcome.report().request_id().map(str::to_owned)
}

#[tokio::test]
async fn given_a_request_id_header_when_run_then_the_report_carries_it() {
    let id = request_id_for(vec![(
        "request-id",
        b"req_018EeWyXxfu5pfWkrYcMdjWG".to_vec(),
    )])
    .await;
    assert_eq!(id.as_deref(), Some("req_018EeWyXxfu5pfWkrYcMdjWG"));
}

#[tokio::test]
async fn given_an_empty_missing_or_invalid_request_id_when_run_then_the_report_has_none() {
    assert_eq!(request_id_for(vec![("request-id", Vec::new())]).await, None);
    assert_eq!(request_id_for(vec![]).await, None);
    assert_eq!(
        request_id_for(vec![("request-id", vec![0xff, 0xfe, b'x'])]).await,
        None
    );
    assert_eq!(
        request_id_for(vec![("x-request-id", b"req_9".to_vec())]).await,
        None
    );
}

#[tokio::test]
async fn given_a_refusal_with_a_request_id_when_run_then_the_report_carries_it_too() {
    let called = call_with(plain_dialect(), &hello(), |_| {
        http("401 Unauthorized", &[("request-id", "req_77")], b"{}")
    })
    .await;
    assert_eq!(called.outcome.report().request_id(), Some("req_77"));
}

#[tokio::test]
async fn given_one_dialect_when_two_calls_run_at_once_then_each_gets_its_own_outcome() {
    let server = TestServer::start(|index| {
        let body = if index.is_multiple_of(2) {
            BASIC.to_owned()
        } else {
            BASIC.replace("Hello", "Howdy")
        };
        http("200 OK", &[], body.as_bytes())
    })
    .await;
    let spec = Arc::new(spec(plain_dialect(), &server));
    let first = {
        let spec = Arc::clone(&spec);
        tokio::spawn(async move {
            let mut sink = crate::test_sink::RecordingSink::default();
            br_llm_clients_common_call::run(&spec, &hello(), &mut sink).await
        })
    };
    let second = {
        let spec = Arc::clone(&spec);
        tokio::spawn(async move {
            let mut sink = crate::test_sink::RecordingSink::default();
            br_llm_clients_common_call::run(&spec, &hello(), &mut sink).await
        })
    };
    let outcomes = [first.await.expect("first"), second.await.expect("second")];
    let mut texts: Vec<String> = outcomes
        .iter()
        .map(
            |outcome| match outcome.result().expect("a step").content() {
                [AssistantBlock::Text { text }] => text.as_str().to_owned(),
                other => panic!("unexpected content {other:?}"),
            },
        )
        .collect();
    texts.sort();
    assert_eq!(texts, ["Hello!", "Howdy!"]);
    for outcome in &outcomes {
        assert_eq!(
            outcome.result().expect("a step").stop_reason(),
            &StopReason::EndTurn
        );
    }
}
