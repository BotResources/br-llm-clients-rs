use br_llm_clients_core::ClientError;
use br_llm_messages::{ContentBlock, ResponseEnding};
use serde_json::{Value, json};

use crate::run_support::{arguments_of, call_stream, call_structured_stream, calls_of, texts_of};
use crate::test_support::{chunk, content_chunk, finish_chunk, shape, tool_chunk, usage_chunk};

fn begin(index: u64, id: &str, name: &str) -> String {
    tool_chunk(
        json!([{"index": index, "id": id, "type": "function", "function": {"name": name, "arguments": ""}}]),
        None,
    )
}

fn piece(index: u64, text: &str) -> String {
    tool_chunk(
        json!([{"index": index, "function": {"arguments": text}}]),
        None,
    )
}

#[tokio::test]
async fn given_one_streamed_tool_call_when_called_then_the_response_holds_the_call_and_the_tool_use_ending()
 {
    let called = call_stream(&[
        chunk(json!({"role": "assistant", "content": null}), None),
        begin(0, "call_abc", "get_weather"),
        piece(0, "{\"loc"),
        piece(0, "ation\":\"Paris\"}"),
        finish_chunk("tool_calls"),
        usage_chunk(json!({"prompt_tokens": 50, "completion_tokens": 20})),
    ])
    .await;
    let response = called.response();
    let calls = calls_of(response);
    assert_eq!(calls.len(), 1);
    let call = calls.first().expect("call");
    assert_eq!(call.external_id().map(|id| id.as_str()), Some("call_abc"));
    assert_eq!(call.name().as_str(), "get_weather");
    assert_eq!(arguments_of(call), json!({"location": "Paris"}));
    assert_eq!(response.ending(), &ResponseEnding::tool_use());
}

#[tokio::test]
async fn given_a_name_in_a_later_fragment_when_called_then_the_call_is_complete() {
    let called = call_stream(&[
        tool_chunk(json!([{"index": 0, "id": "c1", "type": "function", "function": {"arguments": "{\"a\":"}}]), None),
        tool_chunk(json!([{"index": 0, "function": {"name": "late_name", "arguments": "1}"}}]), None),
        finish_chunk("tool_calls"),
    ])
    .await;
    let calls = calls_of(called.response());
    assert_eq!(
        calls.first().map(|call| call.name().as_str()),
        Some("late_name")
    );
    assert_eq!(
        calls.first().map(|call| arguments_of(call)),
        Some(json!({"a": 1}))
    );
}

#[tokio::test]
async fn given_two_parallel_calls_when_called_then_both_are_in_the_response_in_index_order() {
    let called = call_stream(&[
        begin(0, "call_a", "first"),
        begin(1, "call_b", "second"),
        piece(1, "{\"b\":2}"),
        piece(0, "{\"a\":1}"),
        finish_chunk("tool_calls"),
    ])
    .await;
    let calls: Vec<(String, Value)> = calls_of(called.response())
        .into_iter()
        .map(|call| (call.name().as_str().to_owned(), arguments_of(call)))
        .collect();
    assert_eq!(
        calls,
        [
            ("first".to_owned(), json!({"a": 1})),
            ("second".to_owned(), json!({"b": 2}))
        ]
    );
}

#[tokio::test]
async fn given_text_and_a_tool_call_when_called_then_the_text_comes_first_and_the_call_after() {
    let called = call_stream(&[
        content_chunk("Let me check. "),
        begin(0, "c1", "lookup"),
        content_chunk("One moment."),
        piece(0, "{}"),
        finish_chunk("tool_calls"),
    ])
    .await;
    let response = called.response();
    assert_eq!(texts_of(response), ["Let me check. One moment."]);
    assert!(matches!(
        response.blocks().last(),
        Some(ContentBlock::RuntimeToolCall(_))
    ));
}

#[tokio::test]
async fn given_blank_arguments_with_a_tool_calls_finish_when_called_then_the_arguments_are_an_empty_object()
 {
    let called = call_stream(&[begin(0, "c1", "no_args"), finish_chunk("tool_calls")]).await;
    let calls = calls_of(called.response());
    assert_eq!(
        calls.first().map(|call| arguments_of(call)),
        Some(json!({}))
    );
}

#[tokio::test]
async fn given_blank_arguments_with_a_length_finish_when_called_then_the_reply_is_unusable() {
    let called = call_stream(&[begin(0, "c1", "no_args"), finish_chunk("length")]).await;
    assert_eq!(
        called.error(),
        &ClientError::ReplyUnusable {
            ending: ResponseEnding::output_exhaustion()
        }
    );
}

#[tokio::test]
async fn given_arguments_cut_by_a_length_finish_when_called_then_the_reply_is_unusable() {
    let called = call_stream(&[
        begin(0, "c1", "f"),
        piece(0, "{\"a\":"),
        finish_chunk("length"),
    ])
    .await;
    assert_eq!(
        called.error(),
        &ClientError::ReplyUnusable {
            ending: ResponseEnding::output_exhaustion()
        }
    );
}

#[tokio::test]
async fn given_a_tool_call_with_no_finish_before_the_sentinel_when_called_then_the_stream_is_truncated()
 {
    let called = call_stream(&[begin(0, "c1", "f"), piece(0, "{}")]).await;
    assert_eq!(called.error(), &ClientError::StreamTruncated);
}

#[tokio::test]
async fn given_no_id_and_no_type_from_the_provider_when_called_then_the_call_has_no_external_id() {
    let called = call_stream(&[
        tool_chunk(
            json!([{"index": 0, "function": {"name": "f", "arguments": "{}"}}]),
            None,
        ),
        finish_chunk("tool_calls"),
    ])
    .await;
    let calls = calls_of(called.response());
    assert_eq!(calls.len(), 1);
    assert!(
        calls
            .first()
            .is_some_and(|call| call.external_id().is_none())
    );
}

#[tokio::test]
async fn given_a_nameless_call_when_called_then_the_reply_is_malformed() {
    let called = call_stream(&[
        tool_chunk(
            json!([{"index": 0, "id": "c1", "function": {"arguments": "{}"}}]),
            None,
        ),
        finish_chunk("tool_calls"),
    ])
    .await;
    assert!(matches!(
        called.error(),
        ClientError::MalformedResponse { .. }
    ));
}

#[tokio::test]
async fn given_a_tool_name_that_is_not_valid_when_called_then_the_reply_is_malformed() {
    let called = call_stream(&[
        begin(0, "c1", "bad name!"),
        piece(0, "{}"),
        finish_chunk("tool_calls"),
    ])
    .await;
    assert!(matches!(
        called.error(),
        ClientError::MalformedResponse { .. }
    ));
}

#[tokio::test]
async fn given_structured_output_with_content_and_a_tool_call_when_called_then_the_reply_is_malformed()
 {
    let called = call_structured_stream(
        &[
            content_chunk("{\"v\":1}"),
            begin(0, "c1", "f"),
            piece(0, "{}"),
            finish_chunk("tool_calls"),
        ],
        shape("answer", None, Vec::new()),
    )
    .await;
    assert!(matches!(
        called.error(),
        ClientError::MalformedResponse { .. }
    ));
}

#[tokio::test]
async fn given_a_reply_of_a_tool_call_with_a_stop_finish_when_called_then_the_core_rule_wins() {
    let called = call_stream(&[begin(0, "c1", "f"), piece(0, "{}"), finish_chunk("stop")]).await;
    assert_eq!(called.response().ending(), &ResponseEnding::tool_use());
}
