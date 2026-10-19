use br_llm_clients_core::ClientError;

use crate::test_call::*;
use crate::test_streams::*;

fn malformed(error: &ClientError) -> bool {
    matches!(error, ClientError::MalformedResponse { .. })
}

#[tokio::test]
async fn given_a_server_tool_block_when_run_then_the_reply_is_malformed() {
    let stream = BASIC.replace(
        r#"{"type": "text", "text": ""}"#,
        r#"{"type": "server_tool_use", "id": "srv_1", "name": "web_search", "input": {}}"#,
    );
    let called = call_stream(&hello(), &stream).await;
    assert!(malformed(called.error()));
}

#[tokio::test]
async fn given_a_delta_of_an_unknown_type_when_run_then_the_reply_is_malformed() {
    let stream = BASIC.replace("text_delta", "citations_delta");
    let called = call_stream(&hello(), &stream).await;
    assert!(malformed(called.error()));
}

#[tokio::test]
async fn given_an_event_before_message_start_when_run_then_the_reply_is_malformed() {
    let cut = BASIC
        .find("event: content_block_start")
        .expect("a block start");
    let called = call_stream(&hello(), &BASIC[cut..]).await;
    assert!(malformed(called.error()));
}

#[tokio::test]
async fn given_data_that_is_not_json_when_run_then_the_reply_is_malformed() {
    let called = call_stream(&hello(), "event: message_start\ndata: not json\n\n").await;
    assert!(malformed(called.error()));
}

#[tokio::test]
async fn given_an_event_name_that_differs_from_the_type_when_run_then_the_reply_is_malformed() {
    let stream = BASIC.replacen("event: ping", "event: message_stop", 1);
    let called = call_stream(&hello(), &stream).await;
    assert!(malformed(called.error()));
}

#[tokio::test]
async fn given_an_unknown_event_type_in_the_stream_when_run_then_the_call_still_succeeds() {
    let extra = "event: future_event\ndata: {\"type\": \"future_event\", \"x\": 1}\n\n";
    let stream = BASIC.replacen("event: ping", &format!("{extra}event: ping"), 1);
    let called = call_stream(&hello(), &stream).await;
    assert!(called.outcome.result().is_ok());
}

#[tokio::test]
async fn given_a_usage_with_a_negative_number_when_run_then_the_reply_is_malformed() {
    let stream = BASIC.replace(r#""output_tokens": 15"#, r#""output_tokens": -15"#);
    let called = call_stream(&hello(), &stream).await;
    assert!(malformed(called.error()));
}

#[tokio::test]
async fn given_a_second_message_start_when_run_then_the_reply_is_malformed() {
    let first = BASIC.split("\n\n").next().expect("first frame").to_owned();
    let stream = format!("{first}\n\n{BASIC}");
    let called = call_stream(&hello(), &stream).await;
    assert!(malformed(called.error()));
}
