use br_llm_messages::{StopReason, StreamEvent, Usage};
use serde_json::{Value, json};

use crate::ReasoningLevel;
use crate::run_support::{
    call_stream, call_structured_stream, call_with, ok_stream, sse_with_sentinel,
};
use crate::test_support::{
    chunk, content_chunk, dialect_for, finish_chunk, hello, options, shape, usage_chunk,
};

fn documented_stream() -> Vec<String> {
    vec![
        r#"{"id":"chatcmpl-123","object":"chat.completion.chunk","created":1694268190,"model":"gpt-6-astra", "system_fingerprint": "fp_44709d6fcb", "choices":[{"index":0,"delta":{"role":"assistant","content":""},"logprobs":null,"finish_reason":null}]}"#.to_owned(),
        r#"{"id":"chatcmpl-123","object":"chat.completion.chunk","created":1694268190,"model":"gpt-6-astra", "system_fingerprint": "fp_44709d6fcb", "choices":[{"index":0,"delta":{"content":"Hello"},"logprobs":null,"finish_reason":null}]}"#.to_owned(),
        r#"{"id":"chatcmpl-123","object":"chat.completion.chunk","created":1694268190,"model":"gpt-6-astra", "system_fingerprint": "fp_44709d6fcb", "choices":[{"index":0,"delta":{},"logprobs":null,"finish_reason":"stop"}]}"#.to_owned(),
        usage_chunk(json!({
            "prompt_tokens": 9, "completion_tokens": 12, "total_tokens": 21,
            "prompt_tokens_details": {"cached_tokens": 4, "audio_tokens": 0},
            "completion_tokens_details": {"reasoning_tokens": 3, "audio_tokens": 0,
                "accepted_prediction_tokens": 0, "rejected_prediction_tokens": 0}
        }))
        .replace("gpt-4o", "gpt-6-astra"),
    ]
}

#[tokio::test]
async fn given_the_documented_text_stream_when_called_then_the_step_holds_the_text_stop_usage_and_model()
 {
    let called = call_stream(&documented_stream()).await;
    let step = called.step();
    let texts: Vec<&str> = step.text().map(|text| text.as_str()).collect();
    assert_eq!(texts, ["Hello"]);
    assert_eq!(step.stop_reason(), &StopReason::EndTurn);
    assert_eq!(
        step.usage(),
        Some(&Usage {
            input_tokens: 9,
            output_tokens: 12,
            cache_read_tokens: Some(4),
            cache_write_tokens: None,
            thinking_tokens: Some(3),
        })
    );
    assert_eq!(
        step.model().map(|model| model.as_str()),
        Some("gpt-6-astra")
    );
    assert_eq!(
        called.outcome.report().model().map(|model| model.as_str()),
        Some("gpt-6-astra")
    );
    assert_eq!(called.outcome.report().usage(), step.usage());
}

#[tokio::test]
async fn given_the_documented_text_stream_when_called_then_the_sink_gets_one_text_block_in_order() {
    let called = call_stream(&documented_stream()).await;
    assert!(matches!(
        called.events.first(),
        Some(StreamEvent::BlockStart { index: 0, .. })
    ));
    assert!(matches!(
        called.events.last(),
        Some(StreamEvent::Finish { .. })
    ));
    let text: String = called
        .events
        .iter()
        .filter_map(|event| match event {
            StreamEvent::TextDelta { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(text, "Hello");
}

#[tokio::test]
async fn given_a_call_when_sent_then_the_request_on_the_wire_is_the_encoded_body_with_the_headers()
{
    let bytes = sse_with_sentinel(&documented_stream());
    let called = call_with(
        dialect_for(ReasoningLevel::Low, options()),
        &hello(),
        move |_| ok_stream(&bytes),
    )
    .await;
    let received = called.server.requests();
    let request = received.first().expect("one request");
    assert!(
        request
            .request_line
            .starts_with("POST /v1/chat/completions")
    );
    assert_eq!(request.header_values("content-type"), ["application/json"]);
    assert_eq!(request.header_values("accept"), ["text/event-stream"]);
    let body: Value = serde_json::from_slice(&request.body).expect("json");
    assert_eq!(body["stream"], json!(true));
    assert_eq!(body["stream_options"], json!({"include_usage": true}));
    assert_eq!(body["reasoning_effort"], json!("low"));
    assert_eq!(
        body["messages"],
        json!([{"role": "user", "content": "hello"}])
    );
}

#[tokio::test]
async fn given_a_refusal_with_a_stop_when_called_then_the_step_is_a_refusal_with_the_text() {
    let called = call_stream(&[
        chunk(json!({"role": "assistant", "refusal": ""}), None),
        chunk(json!({"refusal": "I can't help with that."}), None),
        finish_chunk("stop"),
        usage_chunk(json!({"prompt_tokens": 5, "completion_tokens": 6})),
    ])
    .await;
    let step = called.step();
    assert_eq!(step.stop_reason(), &StopReason::Refusal);
    let texts: Vec<&str> = step.text().map(|text| text.as_str()).collect();
    assert_eq!(texts, ["I can't help with that."]);
}

#[tokio::test]
async fn given_a_blank_leading_text_when_called_then_the_holdback_of_common_call_applies() {
    let called = call_stream(&[
        content_chunk("\n\n"),
        content_chunk("Answer"),
        finish_chunk("stop"),
    ])
    .await;
    let texts: Vec<&str> = called.step().text().map(|text| text.as_str()).collect();
    assert_eq!(texts, ["\n\nAnswer"]);
}

#[tokio::test]
async fn given_a_reply_with_only_blank_text_when_called_then_the_reply_is_unusable_by_the_core_rules()
 {
    let called = call_stream(&[content_chunk("  "), finish_chunk("stop")]).await;
    assert_eq!(
        called.error(),
        &br_llm_clients_core::ClientError::ReplyUnusable {
            stop_reason: StopReason::EndTurn
        }
    );
}

#[tokio::test]
async fn given_a_reasoning_field_when_called_then_the_step_holds_thinking_without_a_signature() {
    let configured = options().with_reasoning_field("reasoning_content");
    let chunks = [
        chunk(json!({"reasoning_content": "weighing"}), None),
        chunk(json!({"reasoning_content": " options"}), None),
        content_chunk("Done"),
        finish_chunk("stop"),
    ];
    let bytes = sse_with_sentinel(&chunks);
    let called = call_with(
        dialect_for(ReasoningLevel::High, configured),
        &hello(),
        move |_| ok_stream(&bytes),
    )
    .await;
    let step = called.step();
    let thinking: Vec<_> = step.thinking().collect();
    assert_eq!(thinking.len(), 1);
    assert_eq!(
        thinking.first().map(|block| block.text.as_str()),
        Some("weighing options")
    );
    assert!(
        thinking
            .first()
            .is_some_and(|block| block.signature.is_none())
    );
}

#[tokio::test]
async fn given_no_reasoning_field_when_a_host_sends_reasoning_then_the_step_holds_no_thinking_and_no_error()
 {
    let called = call_stream(&[
        chunk(json!({"reasoning_content": "private"}), None),
        content_chunk("Done"),
        finish_chunk("stop"),
    ])
    .await;
    assert_eq!(called.step().thinking().count(), 0);
}

#[tokio::test]
async fn given_a_structured_reply_when_called_then_the_step_holds_the_structured_value() {
    let output = shape("answer", None, Vec::new());
    let called = call_structured_stream(
        &[
            content_chunk("{\"value\":"),
            content_chunk("42}"),
            finish_chunk("stop"),
        ],
        output,
    )
    .await;
    let values: Vec<&Value> = called.step().structured().collect();
    assert_eq!(values, [&json!({"value": 42})]);
}

#[tokio::test]
async fn given_a_cut_off_reply_when_called_then_the_max_tokens_stop_is_kept_for_text() {
    let called = call_stream(&[content_chunk("Once upon"), finish_chunk("length")]).await;
    assert_eq!(called.step().stop_reason(), &StopReason::MaxTokens);
}

#[tokio::test]
async fn given_a_content_filter_finish_when_called_then_the_stop_reason_is_content_filter() {
    let called = call_stream(&[content_chunk("Partial"), finish_chunk("content_filter")]).await;
    assert_eq!(called.step().stop_reason(), &StopReason::ContentFilter);
    let texts: Vec<&str> = called.step().text().map(|text| text.as_str()).collect();
    assert_eq!(texts, ["Partial"]);
}

#[tokio::test]
async fn given_a_content_filter_finish_with_no_content_when_called_then_the_content_is_blocked() {
    let called = call_stream(&[finish_chunk("content_filter")]).await;
    assert_eq!(
        called.error(),
        &br_llm_clients_core::ClientError::ContentBlocked
    );
}
