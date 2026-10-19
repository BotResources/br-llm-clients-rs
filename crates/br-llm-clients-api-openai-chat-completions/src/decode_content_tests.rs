use br_llm_clients_common_call::{End, Payload, PayloadDecoder, ProviderBlock, ProviderEvent};
use br_llm_clients_core::OutputMode;
use br_llm_messages::StopReason;
use serde_json::json;

use crate::test_support::{
    chunk, content_chunk, decoder_for, end, feed, finish_chunk, options, plain_decoder, run_chunks,
    shape, start, text_delta, without_model,
};
use crate::{ChatCompletionsDialect, ReasoningLevel};
use br_llm_clients_common_call::Dialect;

fn with_reasoning_field(output: &OutputMode) -> Box<dyn PayloadDecoder> {
    let configured = options().with_reasoning_field("reasoning_content");
    let dialect = ChatCompletionsDialect::new(
        crate::test_support::settings(ReasoningLevel::Off),
        configured,
    )
    .expect("dialect");
    dialect.decoder(output)
}

#[test]
fn given_the_documented_text_stream_when_decoded_then_one_text_block_is_opened_filled_and_closed() {
    let events = run_chunks(&[
        chunk(json!({"role": "assistant", "content": ""}), None),
        content_chunk("Hello"),
        content_chunk(" world"),
        finish_chunk("stop"),
    ]);
    assert_eq!(
        events,
        [
            start("text", ProviderBlock::Text),
            text_delta("text", "Hello"),
            text_delta("text", " world"),
            end("text"),
            ProviderEvent::Stop(StopReason::EndTurn),
        ]
    );
}

#[test]
fn given_an_empty_content_string_when_decoded_then_no_event_is_given() {
    assert_eq!(run_chunks(&[content_chunk("")]), []);
}

#[test]
fn given_the_sentinel_when_decoded_then_complete_is_the_only_event() {
    let mut decoder = plain_decoder();
    assert_eq!(
        feed(decoder.as_mut(), "[DONE]").expect("sentinel"),
        [ProviderEvent::Complete]
    );
}

#[test]
fn given_blank_data_when_decoded_then_no_event_is_given() {
    let mut decoder = plain_decoder();
    for data in ["", " ", "\t\r\n  "] {
        assert_eq!(feed(decoder.as_mut(), data).expect("blank"), []);
    }
}

#[test]
fn given_any_event_name_when_decoded_then_the_name_is_ignored() {
    let mut decoder = plain_decoder();
    let data = content_chunk("x");
    for name in [None, Some("message"), Some("error"), Some("done")] {
        let payload = Payload::new(name.map(str::to_owned), data.clone());
        let events = without_model(decoder.payload(payload).expect("chunk"));
        assert!(
            events
                .iter()
                .any(|event| matches!(event, ProviderEvent::TextDelta { .. }))
        );
    }
}

#[test]
fn given_data_that_is_not_a_json_object_when_decoded_then_it_is_a_payload_error() {
    let mut decoder = plain_decoder();
    for data in [
        "not json", "[1,2]", "42", "\"text\"", "true", "null", "[DONE] ", "{\"a\":", "done",
    ] {
        assert!(feed(decoder.as_mut(), data).is_err(), "{data}");
    }
}

#[test]
fn given_a_payload_error_when_read_then_the_text_never_repeats_the_data() {
    let secret = "sk-very-secret-value";
    let error = crate::test_support::fails(&format!("not json {secret}"));
    assert!(!error.text().contains(secret));
}

#[test]
fn given_the_end_of_the_body_without_the_sentinel_when_ended_then_the_marker_is_missing() {
    let mut decoder = plain_decoder();
    let _ = feed(decoder.as_mut(), &content_chunk("hi")).expect("chunk");
    assert_eq!(decoder.end(), Ok(End::MarkerMissing));
}

#[test]
fn given_the_structured_mode_when_content_arrives_then_it_is_a_structured_block() {
    let mut decoder = decoder_for(&shape("answer", None, Vec::new()));
    let chunks = [
        content_chunk("{\"a\":"),
        content_chunk("1}"),
        finish_chunk("stop"),
    ];
    let events = without_model(crate::test_support::feed_all(decoder.as_mut(), &chunks));
    assert_eq!(
        events,
        [
            start("structured", ProviderBlock::Structured),
            ProviderEvent::StructuredDelta {
                key: "structured".to_owned(),
                json: "{\"a\":".to_owned()
            },
            ProviderEvent::StructuredDelta {
                key: "structured".to_owned(),
                json: "1}".to_owned()
            },
            end("structured"),
            ProviderEvent::Stop(StopReason::EndTurn),
        ]
    );
}

#[test]
fn given_a_refusal_when_decoded_then_it_is_a_text_block_with_its_own_key() {
    let events = run_chunks(&[
        chunk(json!({"refusal": "I can't"}), None),
        chunk(json!({"refusal": " help"}), None),
        finish_chunk("stop"),
    ]);
    assert_eq!(
        events,
        [
            start("refusal", ProviderBlock::Text),
            text_delta("refusal", "I can't"),
            text_delta("refusal", " help"),
            end("refusal"),
            ProviderEvent::Stop(StopReason::Refusal),
        ]
    );
}

#[test]
fn given_content_then_a_refusal_when_decoded_then_the_text_block_is_closed_first() {
    let events = run_chunks(&[content_chunk("a"), chunk(json!({"refusal": "no"}), None)]);
    assert_eq!(
        events,
        [
            start("text", ProviderBlock::Text),
            text_delta("text", "a"),
            end("text"),
            start("refusal", ProviderBlock::Text),
            text_delta("refusal", "no"),
        ]
    );
}

#[test]
fn given_one_delta_with_every_field_when_decoded_then_they_are_read_in_the_documented_order() {
    let mut decoder = with_reasoning_field(&OutputMode::Plain);
    let delta = json!({
        "tool_calls": [{"index": 0, "id": "c1", "type": "function", "function": {"name": "f", "arguments": "{}"}}],
        "refusal": "r",
        "content": "c",
        "reasoning_content": "t"
    });
    let events = without_model(feed(decoder.as_mut(), &chunk(delta, None)).expect("chunk"));
    assert_eq!(
        events,
        [
            start("reasoning", ProviderBlock::Thinking),
            ProviderEvent::ThinkingDelta {
                key: "reasoning".to_owned(),
                text: "t".to_owned()
            },
            end("reasoning"),
            start("text", ProviderBlock::Text),
            text_delta("text", "c"),
            end("text"),
            start("refusal", ProviderBlock::Text),
            text_delta("refusal", "r"),
        ]
    );
}

#[test]
fn given_the_metadata_of_the_documentation_when_decoded_then_it_is_ignored() {
    let data = json!({
        "id": "chatcmpl-123", "object": "chat.completion.chunk", "created": 1694268190,
        "model": "gpt-6-astra", "system_fingerprint": "fp_44709d6fcb", "service_tier": "default",
        "obfuscation": "r4N7vQ2m", "moderation": null, "unknown_future_field": {"a": [1]},
        "choices": [{
            "index": 0, "logprobs": {"content": null, "refusal": null}, "annotations": [],
            "delta": {"role": "assistant", "content": "Hello", "refusal": null, "unknown": 1},
            "finish_reason": null
        }]
    })
    .to_string();
    let mut decoder = plain_decoder();
    let events = without_model(feed(decoder.as_mut(), &data).expect("chunk"));
    assert_eq!(
        events,
        [
            start("text", ProviderBlock::Text),
            text_delta("text", "Hello")
        ]
    );
}
