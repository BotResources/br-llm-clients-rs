use br_llm_clients_common_call::{Dialect, PayloadDecoder, ProviderBlock, ProviderEvent};
use br_llm_clients_core::{ClientError, OutputMode};
use serde_json::{Map, Value, json};

use crate::run_support::{call_refused, call_with, ok_stream, sse_with_sentinel, thinking_of};
use crate::test_support::{
    chunk, content_chunk, dialect_for, encoded_body, end, feed_all, finish_chunk, hello, options,
    start, tool_chunk, without_metadata,
};
use crate::{ChatCompletionsOptions, MaxTokensField, ReasoningLevel};

fn decoder_with(configured: ChatCompletionsOptions) -> Box<dyn PayloadDecoder> {
    dialect_for(ReasoningLevel::Off, configured).decoder(&OutputMode::Plain)
}

fn thinking_texts(events: &[ProviderEvent]) -> Vec<&str> {
    events
        .iter()
        .filter_map(|event| match event {
            ProviderEvent::ThinkingDelta { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
fn given_the_reasoning_field_named_reasoning_when_decoded_then_its_text_is_thinking() {
    let mut decoder = decoder_with(options().with_reasoning_field("reasoning"));
    let events = without_metadata(feed_all(
        decoder.as_mut(),
        &[
            chunk(json!({"reasoning": "first, "}), None),
            chunk(json!({"reasoning": "then"}), None),
            content_chunk("ok"),
        ],
    ));
    assert_eq!(
        events.first(),
        Some(&start("reasoning", ProviderBlock::Thinking))
    );
    assert_eq!(thinking_texts(&events), ["first, ", "then"]);
}

#[test]
fn given_a_chunk_holding_both_reasoning_fields_with_the_same_text_when_decoded_then_only_the_configured_one_is_read()
 {
    let both = chunk(
        json!({"reasoning_content": "weighing", "reasoning": "weighing"}),
        None,
    );
    for field in ["reasoning_content", "reasoning"] {
        let mut decoder = decoder_with(options().with_reasoning_field(field));
        let events = without_metadata(feed_all(decoder.as_mut(), std::slice::from_ref(&both)));
        assert_eq!(thinking_texts(&events), ["weighing"], "{field}");
    }
    let mut unset = decoder_with(options());
    assert!(without_metadata(feed_all(unset.as_mut(), &[both])).is_empty());
}

#[test]
fn given_both_reasoning_fields_when_the_other_one_is_not_a_string_then_it_is_metadata_and_ignored()
{
    let mut decoder = decoder_with(options().with_reasoning_field("reasoning_content"));
    let data = chunk(
        json!({"reasoning_content": "weighing", "reasoning": {"summary": 1}}),
        None,
    );
    let events = without_metadata(feed_all(decoder.as_mut(), &[data]));
    assert_eq!(thinking_texts(&events), ["weighing"]);
}

#[test]
fn given_a_tool_fragment_with_no_type_and_no_id_when_decoded_then_the_call_is_given_without_an_id()
{
    let mut decoder = decoder_with(options());
    let events = without_metadata(feed_all(
        decoder.as_mut(),
        &[
            tool_chunk(
                json!([{"index": 0, "function": {"name": "lookup", "arguments": "{\"q\":1}"}}]),
                None,
            ),
            finish_chunk("tool_calls"),
        ],
    ));
    assert_eq!(
        events.first(),
        Some(&start(
            "0",
            ProviderBlock::ToolCall {
                name: "lookup".to_owned(),
                id: None
            }
        ))
    );
    assert_eq!(events.get(2), Some(&end("0")));
}

#[test]
fn given_a_nested_extra_body_when_encoded_then_it_is_written_at_the_top_level_unchanged() {
    let extra: Map<String, Value> = json!({
        "chat_template_kwargs": {"enable_thinking": false},
        "repetition_penalty": 1.05
    })
    .as_object()
    .cloned()
    .expect("object");
    let configured = options().with_extra_body(extra);
    let body = encoded_body(&dialect_for(ReasoningLevel::Off, configured), &hello());
    assert_eq!(
        body["chat_template_kwargs"],
        json!({"enable_thinking": false})
    );
    assert_eq!(body["repetition_penalty"], json!(1.05));
}

#[test]
fn given_max_tokens_and_a_reasoning_content_carry_back_when_encoded_then_the_host_shape_is_written()
{
    let configured = options()
        .with_max_tokens_field(MaxTokensField::MaxTokens)
        .with_reasoning_field("reasoning_content")
        .with_reasoning_carry_back_field("reasoning_content");
    let body = encoded_body(&dialect_for(ReasoningLevel::Off, configured), &hello());
    assert_eq!(body["max_tokens"], json!(1024));
    assert!(body.get("max_completion_tokens").is_none());
}

#[tokio::test]
async fn given_a_host_sending_reasoning_when_called_with_that_field_then_the_response_holds_its_thinking()
 {
    let chunks = [
        chunk(json!({"reasoning": "a"}), None),
        chunk(json!({"reasoning_content": "b", "reasoning": "b"}), None),
        content_chunk("done"),
        finish_chunk("stop"),
    ];
    let bytes = sse_with_sentinel(&chunks);
    let configured = options().with_reasoning_field("reasoning");
    let called = call_with(
        dialect_for(ReasoningLevel::Low, configured),
        &hello(),
        move |_| ok_stream(&bytes),
    )
    .await;
    let thinking = thinking_of(called.response());
    assert_eq!(thinking.first().map(|block| block.text()), Some("ab"));
}

#[tokio::test]
async fn given_each_quota_code_on_a_429_when_called_then_the_quota_is_exceeded() {
    for code in [
        "credit_balance_exhausted",
        "organization_spend_limit_exceeded",
        "project_spend_limit_exceeded",
        "organization_usage_limit_exceeded",
    ] {
        let body = json!({"error": {"message": "m", "type": "billing", "code": code}}).to_string();
        let called = call_refused("429 Too Many Requests", &[], &body).await;
        assert_eq!(called.error(), &ClientError::QuotaExceeded, "{code}");
    }
}
