use br_llm_clients_common_call::{Dialect, PayloadDecoder, ProviderEvent};
use br_llm_clients_core::OutputMode;
use br_llm_messages::Usage;
use serde_json::{Value, json};

use crate::ReasoningLevel;
use crate::test_support::{
    chunk, dialect_for, fails, feed, options, plain_decoder, usage_chunk, without_metadata,
};

pub(crate) fn usages(decoder: &mut dyn PayloadDecoder, data: &str) -> Vec<Usage> {
    feed(decoder, data)
        .expect("chunk")
        .into_iter()
        .filter_map(|event| match event {
            ProviderEvent::Usage(usage) => Some(usage),
            _ => None,
        })
        .collect()
}

pub(crate) fn one_usage(usage: Value) -> Option<Usage> {
    let mut decoder = plain_decoder();
    usages(decoder.as_mut(), &usage_chunk(usage))
        .into_iter()
        .next()
}

pub(crate) fn with_cache_field() -> Box<dyn PayloadDecoder> {
    let configured = options().with_cache_read_field("cached_prompt_tokens");
    dialect_for(ReasoningLevel::Off, configured).decoder(&OutputMode::Plain)
}

#[test]
fn given_the_documented_usage_chunk_when_decoded_then_every_source_field_is_read() {
    let usage = json!({
        "prompt_tokens": 100, "completion_tokens": 40, "total_tokens": 140,
        "prompt_tokens_details": {"cached_tokens": 30, "audio_tokens": 0, "cache_write_tokens": 20},
        "completion_tokens_details": {
            "reasoning_tokens": 25, "audio_tokens": 0,
            "accepted_prediction_tokens": 0, "rejected_prediction_tokens": 0
        }
    });
    assert_eq!(
        one_usage(usage),
        Some(
            Usage::of_input_tokens(100)
                .with_output_tokens(40)
                .with_cache_read_input_tokens(30)
                .with_cache_creation_input_tokens(20)
                .with_thinking_tokens(25)
        )
    );
}

#[test]
fn given_only_the_two_required_counts_when_decoded_then_the_other_fields_are_absent() {
    assert_eq!(
        one_usage(json!({"prompt_tokens": 9, "completion_tokens": 12, "total_tokens": 21})),
        Some(Usage::of_input_tokens(9).with_output_tokens(12))
    );
}

#[test]
fn given_reported_zeros_when_decoded_then_they_stay_zero() {
    let usage = json!({
        "prompt_tokens": 0, "completion_tokens": 0,
        "prompt_tokens_details": {"cached_tokens": 0, "cache_write_tokens": 0},
        "completion_tokens_details": {"reasoning_tokens": 0}
    });
    assert_eq!(
        one_usage(usage),
        Some(
            Usage::of_input_tokens(0)
                .with_output_tokens(0)
                .with_cache_read_input_tokens(0)
                .with_cache_creation_input_tokens(0)
                .with_thinking_tokens(0)
        )
    );
}

#[test]
fn given_only_the_prompt_count_when_decoded_then_a_usage_holds_it_with_its_cache_counts() {
    for usage in [
        json!({"prompt_tokens": 5, "prompt_tokens_details": {"cached_tokens": 2, "cache_write_tokens": 1}}),
        json!({"prompt_tokens": 5, "completion_tokens": null, "prompt_tokens_details": {"cached_tokens": 2, "cache_write_tokens": 1}}),
    ] {
        assert_eq!(
            one_usage(usage.clone()),
            Some(
                Usage::of_input_tokens(5)
                    .with_cache_read_input_tokens(2)
                    .with_cache_creation_input_tokens(1)
            ),
            "{usage}"
        );
    }
}

#[test]
fn given_only_the_completion_count_when_decoded_then_a_usage_holds_it_with_its_thinking_count() {
    for usage in [
        json!({"completion_tokens": 5, "completion_tokens_details": {"reasoning_tokens": 3}}),
        json!({"prompt_tokens": null, "completion_tokens": 5, "completion_tokens_details": {"reasoning_tokens": 3}}),
    ] {
        assert_eq!(
            one_usage(usage.clone()),
            Some(Usage::of_output_tokens(5).with_thinking_tokens(3)),
            "{usage}"
        );
    }
}

#[test]
fn given_a_thinking_count_without_the_completion_count_when_decoded_then_it_is_not_given() {
    let usage = json!({
        "prompt_tokens": 5,
        "completion_tokens_details": {"reasoning_tokens": 3}
    });
    assert_eq!(one_usage(usage), Some(Usage::of_input_tokens(5)));
}

#[test]
fn given_a_usage_with_neither_count_when_decoded_then_no_usage_is_given() {
    for usage in [
        json!({}),
        json!({"prompt_tokens": null, "completion_tokens": null}),
        json!({"total_tokens": 5, "prompt_tokens_details": {"cached_tokens": 2}, "completion_tokens_details": {"reasoning_tokens": 1}}),
    ] {
        assert_eq!(one_usage(usage.clone()), None, "{usage}");
    }
}

#[test]
fn given_a_null_or_absent_usage_when_decoded_then_no_usage_is_given() {
    let mut decoder = plain_decoder();
    let with_null =
        chunk(json!({"content": "x"}), None).replace("\"created\"", "\"usage\":null,\"created\"");
    assert!(usages(decoder.as_mut(), &with_null).is_empty());
    assert!(usages(decoder.as_mut(), &chunk(json!({"content": "x"}), None)).is_empty());
}

#[test]
fn given_a_usage_on_several_chunks_when_decoded_then_one_usage_is_given_for_each() {
    let mut decoder = plain_decoder();
    let first = usages(
        decoder.as_mut(),
        &usage_chunk(json!({"prompt_tokens": 1, "completion_tokens": 1})),
    );
    let second = usages(
        decoder.as_mut(),
        &usage_chunk(json!({"prompt_tokens": 1, "completion_tokens": 7})),
    );
    assert_eq!(first.len(), 1);
    assert_eq!(
        second.first().and_then(|usage| usage.output_tokens()),
        Some(7)
    );
}

#[test]
fn given_a_usage_on_a_chunk_with_a_delta_when_decoded_then_it_is_given_beside_the_delta_events() {
    let data = chunk(json!({"content": "x"}), None).replace(
        "\"created\"",
        "\"usage\":{\"prompt_tokens\":2,\"completion_tokens\":3},\"created\"",
    );
    let mut decoder = plain_decoder();
    let events = without_metadata(feed(decoder.as_mut(), &data).expect("chunk"));
    assert_eq!(events.len(), 3);
    assert!(matches!(events.last(), Some(ProviderEvent::Usage(_))));
}

#[test]
fn given_a_count_that_is_not_a_non_negative_integer_when_decoded_then_the_field_is_named() {
    let bad = [
        json!(-1),
        json!(1.5),
        json!("3"),
        json!(true),
        json!([1]),
        json!({}),
        json!(1e30),
    ];
    let places = [
        ("prompt_tokens", "prompt_tokens"),
        ("completion_tokens", "completion_tokens"),
    ];
    for value in &bad {
        for (field, named) in places {
            let mut usage = json!({"prompt_tokens": 1, "completion_tokens": 1});
            usage[field] = value.clone();
            assert!(
                fails(&usage_chunk(usage)).text().contains(named),
                "{field} {value}"
            );
        }
        for (details, field) in [
            ("prompt_tokens_details", "cached_tokens"),
            ("prompt_tokens_details", "cache_write_tokens"),
            ("completion_tokens_details", "reasoning_tokens"),
        ] {
            let usage =
                json!({"prompt_tokens": 1, "completion_tokens": 1, details: {field: value}});
            assert!(
                fails(&usage_chunk(usage)).text().contains(field),
                "{field} {value}"
            );
        }
    }
}

#[test]
fn given_a_bad_count_beside_a_missing_one_when_decoded_then_it_is_still_a_payload_error() {
    assert!(
        fails(&usage_chunk(json!({"prompt_tokens": -5})))
            .text()
            .contains("prompt_tokens")
    );
}

#[test]
fn given_a_usage_or_details_of_the_wrong_type_when_decoded_then_the_field_is_named() {
    for value in [json!(1), json!("x"), json!([1]), json!(true)] {
        assert!(
            fails(&usage_chunk(value.clone())).text().contains("usage"),
            "{value}"
        );
        for details in ["prompt_tokens_details", "completion_tokens_details"] {
            let usage = json!({"prompt_tokens": 1, "completion_tokens": 1, details: value});
            assert!(
                fails(&usage_chunk(usage)).text().contains(details),
                "{details} {value}"
            );
        }
    }
}

#[test]
fn given_total_tokens_of_any_shape_when_decoded_then_it_is_not_read() {
    let usage = json!({"prompt_tokens": 1, "completion_tokens": 2, "total_tokens": "many"});
    assert_eq!(
        one_usage(usage).and_then(|usage| usage.output_tokens()),
        Some(2)
    );
}

#[test]
fn given_nothing_is_added_or_subtracted_when_decoded_then_the_counts_are_as_the_wire_gives_them() {
    let usage = json!({
        "prompt_tokens": 10, "completion_tokens": 5,
        "prompt_tokens_details": {"cached_tokens": 8},
        "completion_tokens_details": {"reasoning_tokens": 4}
    });
    let found = one_usage(usage).expect("usage");
    assert_eq!(
        (found.input_tokens(), found.output_tokens()),
        (Some(10), Some(5))
    );
    assert_eq!(found.cache_read_input_tokens(), Some(8));
    assert_eq!(found.thinking_tokens(), Some(4));
}
