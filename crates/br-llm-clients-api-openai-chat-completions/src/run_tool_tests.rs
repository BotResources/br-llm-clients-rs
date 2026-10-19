use br_llm_clients_core::ClientError;
use br_llm_messages::StopReason;
use serde_json::{Value, json};

use crate::run_support::{call_stream, call_structured_stream};
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
async fn given_one_streamed_tool_call_when_called_then_the_step_holds_the_call_and_awaits_results()
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
    let step = called.step();
    let calls: Vec<_> = step.tool_calls().collect();
    assert_eq!(calls.len(), 1);
    let call = calls.first().expect("call");
    assert_eq!(call.id.as_str(), "call_abc");
    assert_eq!(call.name.as_str(), "get_weather");
    assert_eq!(call.arguments, json!({"location": "Paris"}));
    assert_eq!(step.stop_reason(), &StopReason::AwaitingToolResults);
}

#[tokio::test]
async fn given_a_name_in_a_later_fragment_when_called_then_the_call_is_complete() {
    let called = call_stream(&[
        tool_chunk(json!([{"index": 0, "id": "c1", "type": "function", "function": {"arguments": "{\"a\":"}}]), None),
        tool_chunk(json!([{"index": 0, "function": {"name": "late_name", "arguments": "1}"}}]), None),
        finish_chunk("tool_calls"),
    ])
    .await;
    let calls: Vec<_> = called.step().tool_calls().collect();
    assert_eq!(
        calls.first().map(|call| call.name.as_str()),
        Some("late_name")
    );
    assert_eq!(
        calls.first().map(|call| call.arguments.clone()),
        Some(json!({"a": 1}))
    );
}

#[tokio::test]
async fn given_two_parallel_calls_when_called_then_both_are_in_the_step_in_index_order() {
    let called = call_stream(&[
        begin(0, "call_a", "first"),
        begin(1, "call_b", "second"),
        piece(1, "{\"b\":2}"),
        piece(0, "{\"a\":1}"),
        finish_chunk("tool_calls"),
    ])
    .await;
    let calls: Vec<(String, Value)> = called
        .step()
        .tool_calls()
        .map(|call| (call.name.as_str().to_owned(), call.arguments.clone()))
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
    let step = called.step();
    let texts: Vec<&str> = step.text().map(|text| text.as_str()).collect();
    assert_eq!(texts, ["Let me check. One moment."]);
    assert!(
        step.content()
            .last()
            .is_some_and(|block| block.is_tool_call())
    );
}

#[tokio::test]
async fn given_blank_arguments_with_a_tool_calls_finish_when_called_then_the_arguments_are_an_empty_object()
 {
    let called = call_stream(&[begin(0, "c1", "no_args"), finish_chunk("tool_calls")]).await;
    let call = called.step().tool_calls().next().expect("call").clone();
    assert_eq!(call.arguments, json!({}));
}

#[tokio::test]
async fn given_blank_arguments_with_a_length_finish_when_called_then_the_reply_is_unusable() {
    let called = call_stream(&[begin(0, "c1", "no_args"), finish_chunk("length")]).await;
    assert_eq!(
        called.error(),
        &ClientError::ReplyUnusable {
            stop_reason: StopReason::MaxTokens
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
            stop_reason: StopReason::MaxTokens
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
async fn given_no_id_from_the_provider_when_called_then_common_call_generates_one() {
    let called = call_stream(&[
        tool_chunk(
            json!([{"index": 0, "function": {"name": "f", "arguments": "{}"}}]),
            None,
        ),
        finish_chunk("tool_calls"),
    ])
    .await;
    let call = called.step().tool_calls().next().expect("call").clone();
    assert!(br_llm_clients_common_call::is_generated_id(
        call.id.as_str()
    ));
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
    assert_eq!(
        called.step().stop_reason(),
        &StopReason::AwaitingToolResults
    );
}
