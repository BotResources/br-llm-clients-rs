use br_llm_clients_common_call::ProviderEvent;
use serde_json::{Value, json};

use crate::test_support::{chunk, content_chunk, feed, plain_decoder};

fn error_events(data: &str) -> Vec<ProviderEvent> {
    let mut decoder = plain_decoder();
    feed(decoder.as_mut(), data).expect("a chunk")
}

fn only_error(data: &str) -> br_llm_clients_common_call::ErrorInfo {
    match error_events(data).as_slice() {
        [ProviderEvent::Error(info)] => info.clone(),
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn given_an_error_object_when_decoded_then_the_text_is_its_json_and_the_code_is_its_code() {
    let error = json!({
        "message": "The server had an error", "type": "server_error", "param": null, "code": "internal"
    });
    let info = only_error(&json!({"error": error}).to_string());
    assert_eq!(
        serde_json::from_str::<Value>(info.text()).expect("json"),
        error
    );
    assert_eq!(info.code(), Some("internal"));
    assert_eq!(info.status(), None);
    assert_eq!(info.retry_after(), None);
}

#[test]
fn given_an_error_object_with_a_null_code_when_decoded_then_the_type_is_the_code() {
    let data = json!({"error": {"message": "m", "type": "server_error", "code": null}}).to_string();
    assert_eq!(only_error(&data).code(), Some("server_error"));
}

#[test]
fn given_an_error_object_with_a_numeric_code_when_decoded_then_the_type_is_the_code() {
    let data = json!({"error": {"message": "m", "type": "t", "code": 429}}).to_string();
    assert_eq!(only_error(&data).code(), Some("t"));
}

#[test]
fn given_an_error_object_with_neither_string_when_decoded_then_there_is_no_code() {
    let data = json!({"error": {"message": "m", "type": 5, "code": 1}}).to_string();
    assert_eq!(only_error(&data).code(), None);
}

#[test]
fn given_an_error_that_is_a_string_when_decoded_then_the_text_is_the_string_itself() {
    let info = only_error(&json!({"error": "overloaded"}).to_string());
    assert_eq!(info.text(), "overloaded");
    assert_eq!(info.code(), None);
}

#[test]
fn given_an_error_of_another_json_type_when_decoded_then_the_text_is_its_json_and_no_code() {
    for error in [json!(503), json!([1, "a"]), json!(true)] {
        let info = only_error(&json!({"error": error}).to_string());
        assert_eq!(info.text(), error.to_string());
        assert_eq!(info.code(), None);
    }
}

#[test]
fn given_a_null_error_when_decoded_then_the_chunk_is_read_as_usual() {
    let data =
        chunk(json!({"content": "x"}), None).replace("\"created\"", "\"error\":null,\"created\"");
    let events = error_events(&data);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, ProviderEvent::TextDelta { .. }))
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, ProviderEvent::Error(_)))
    );
}

#[test]
fn given_an_error_beside_other_fields_when_decoded_then_nothing_else_is_given_for_the_chunk() {
    let data = json!({
        "model": "gpt-4o",
        "error": {"message": "m", "type": "t"},
        "choices": [{"index": 0, "delta": {"content": "x"}, "finish_reason": "stop"}],
        "usage": {"prompt_tokens": 1, "completion_tokens": 1}
    })
    .to_string();
    assert_eq!(error_events(&data).len(), 1);
}

#[test]
fn given_an_error_beside_malformed_fields_when_decoded_then_the_error_is_still_given() {
    let data = json!({
        "error": {"message": "m", "type": "t"},
        "choices": "broken", "model": 5, "usage": [1]
    })
    .to_string();
    assert_eq!(error_events(&data).len(), 1);
}

#[test]
fn given_an_error_chunk_when_decoded_then_a_later_chunk_is_still_readable() {
    let mut decoder = plain_decoder();
    feed(decoder.as_mut(), &json!({"error": "x"}).to_string()).expect("error");
    assert!(feed(decoder.as_mut(), &content_chunk("y")).is_ok());
}
