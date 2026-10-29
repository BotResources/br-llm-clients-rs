use br_llm_clients_common_call::{PayloadDecoder, ProviderBlock, ProviderEvent};
use br_llm_clients_core::OutputMode;
use serde_json::json;

use crate::test_support::{
    chunk, content_chunk, end, feed, options, run_chunks, start, text_delta, without_metadata,
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

fn reasoning_chunk(value: &str) -> String {
    chunk(json!({"reasoning_content": value}), None)
}

#[test]
fn given_a_reasoning_field_in_the_options_when_it_carries_text_then_a_thinking_block_is_opened() {
    let mut decoder = with_reasoning_field(&OutputMode::Plain);
    let chunks = [
        reasoning_chunk("step one"),
        reasoning_chunk(" and two"),
        content_chunk("answer"),
    ];
    let events = without_metadata(crate::test_support::feed_all(decoder.as_mut(), &chunks));
    assert_eq!(
        events,
        [
            start("reasoning", ProviderBlock::Thinking),
            ProviderEvent::ThinkingDelta {
                key: "reasoning".to_owned(),
                text: "step one".to_owned()
            },
            ProviderEvent::ThinkingDelta {
                key: "reasoning".to_owned(),
                text: " and two".to_owned()
            },
            end("reasoning"),
            start("text", ProviderBlock::Text),
            text_delta("text", "answer"),
        ]
    );
}

#[test]
fn given_no_reasoning_field_in_the_options_when_a_host_sends_one_then_it_is_metadata() {
    let events = run_chunks(&[reasoning_chunk("hidden"), content_chunk("answer")]);
    assert_eq!(
        events,
        [
            start("text", ProviderBlock::Text),
            text_delta("text", "answer")
        ]
    );
}

#[test]
fn given_a_reasoning_field_when_its_value_is_empty_or_null_then_nothing_is_given() {
    let mut decoder = with_reasoning_field(&OutputMode::Plain);
    for data in [
        reasoning_chunk(""),
        chunk(json!({"reasoning_content": null}), None),
    ] {
        assert_eq!(
            without_metadata(feed(decoder.as_mut(), &data).expect("chunk")),
            []
        );
    }
}

#[test]
fn given_a_reasoning_field_when_its_value_is_not_a_string_then_the_field_is_named() {
    let mut decoder = with_reasoning_field(&OutputMode::Plain);
    for value in [json!(1), json!(["a"]), json!({"a": 1}), json!(true)] {
        let data = chunk(json!({"reasoning_content": value}), None);
        let error = feed(decoder.as_mut(), &data).expect_err("wrong type");
        assert!(error.text().contains("reasoning_content"), "{error}");
    }
}

#[test]
fn given_text_then_reasoning_then_text_when_decoded_then_the_second_text_is_a_second_block() {
    let mut decoder = with_reasoning_field(&OutputMode::Plain);
    let chunks = [content_chunk("a"), reasoning_chunk("r"), content_chunk("b")];
    let events = without_metadata(crate::test_support::feed_all(decoder.as_mut(), &chunks));
    assert_eq!(
        events,
        [
            start("text", ProviderBlock::Text),
            text_delta("text", "a"),
            end("text"),
            start("reasoning", ProviderBlock::Thinking),
            ProviderEvent::ThinkingDelta {
                key: "reasoning".to_owned(),
                text: "r".to_owned()
            },
            end("reasoning"),
            start("text", ProviderBlock::Text),
            text_delta("text", "b"),
        ]
    );
}
