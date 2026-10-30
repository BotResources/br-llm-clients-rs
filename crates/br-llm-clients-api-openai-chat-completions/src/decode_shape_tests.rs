use br_llm_clients_common_call::{Dialect, Payload, PayloadDecoder};
use br_llm_clients_core::OutputMode;
use serde_json::{Value, json};

use crate::ReasoningLevel;
use crate::test_support::{chunk, dialect_for, fails, feed, options, plain_decoder};

fn wrong_values() -> Vec<Value> {
    vec![
        json!(1),
        json!(-1),
        json!(1.5),
        json!(true),
        json!("text"),
        json!([]),
        json!([1, "a", null]),
        json!({}),
        json!({"a": {"b": [1]}}),
    ]
}

fn configured_decoder() -> Box<dyn PayloadDecoder> {
    let configured = options()
        .with_reasoning_field("reasoning_content")
        .with_cache_read_field("cached");
    dialect_for(ReasoningLevel::Off, configured).decoder(&OutputMode::Plain)
}

type Place = (&'static str, Box<dyn Fn(Value) -> Value>);

fn named_places() -> Vec<Place> {
    vec![
        ("choices", Box::new(|value| json!({"choices": value}))),
        ("choices[]", Box::new(|value| json!({"choices": [value]}))),
        (
            "index",
            Box::new(|value| json!({"choices": [{"index": value, "delta": {}}]})),
        ),
        (
            "delta",
            Box::new(|value| json!({"choices": [{"delta": value}]})),
        ),
        (
            "content",
            Box::new(|value| json!({"choices": [{"delta": {"content": value}}]})),
        ),
        (
            "refusal",
            Box::new(|value| json!({"choices": [{"delta": {"refusal": value}}]})),
        ),
        (
            "tool_calls",
            Box::new(|value| json!({"choices": [{"delta": {"tool_calls": value}}]})),
        ),
        (
            "choices[].delta.tool_calls[]",
            Box::new(|value| json!({"choices": [{"delta": {"tool_calls": [value]}}]})),
        ),
        (
            "finish_reason",
            Box::new(|value| json!({"choices": [{"delta": {}, "finish_reason": value}]})),
        ),
        ("model", Box::new(|value| json!({"model": value}))),
        ("usage", Box::new(|value| json!({"usage": value}))),
        (
            "reasoning_content",
            Box::new(|value| json!({"choices": [{"delta": {"reasoning_content": value}}]})),
        ),
        (
            "cached",
            Box::new(
                |value| json!({"usage": {"prompt_tokens": 1, "completion_tokens": 1, "cached": value}}),
            ),
        ),
    ]
}

#[test]
fn given_every_named_place_with_any_json_value_when_decoded_then_there_is_never_a_panic() {
    for (place, build) in named_places() {
        for value in wrong_values() {
            let data = build(value.clone()).to_string();
            let mut decoder = configured_decoder();
            let result = decoder.payload(Payload::new(None, data));
            if let Err(error) = result {
                assert!(!error.text().is_empty(), "{place}");
            }
            let _ = decoder.end();
        }
    }
}

#[test]
fn given_a_named_text_place_with_a_non_string_when_decoded_then_the_error_names_the_place() {
    let expectations = [
        ("choices", "choices"),
        ("delta", "delta"),
        ("content", "content"),
        ("refusal", "refusal"),
        ("tool_calls", "tool_calls"),
        ("finish_reason", "finish_reason"),
        ("model", "model"),
        ("usage", "usage"),
        ("reasoning_content", "reasoning_content"),
    ];
    for (place, named) in expectations {
        let build = named_places()
            .into_iter()
            .find(|(candidate, _)| *candidate == place)
            .map(|(_, build)| build)
            .expect("place");
        let data = build(json!(12345)).to_string();
        let mut decoder = configured_decoder();
        let error = feed(decoder.as_mut(), &data).expect_err(place);
        assert!(error.text().contains(named), "{place}: {error}");
    }
}

#[test]
fn given_a_missing_or_null_choices_when_decoded_then_it_is_an_empty_array() {
    let mut decoder = plain_decoder();
    for data in [json!({}), json!({"choices": null}), json!({"choices": []})] {
        assert_eq!(
            feed(decoder.as_mut(), &data.to_string()).expect("chunk"),
            []
        );
    }
}

#[test]
fn given_a_choice_with_no_delta_or_a_null_one_when_decoded_then_it_has_an_empty_delta() {
    let mut decoder = plain_decoder();
    for choice in [
        json!({}),
        json!({"index": 0}),
        json!({"delta": null, "index": null}),
    ] {
        let data = json!({"choices": [choice]}).to_string();
        assert_eq!(feed(decoder.as_mut(), &data).expect("chunk"), []);
    }
}

#[test]
fn given_a_choice_with_no_delta_but_a_finish_when_decoded_then_the_finish_is_read() {
    let mut decoder = plain_decoder();
    let data = json!({"choices": [{"finish_reason": "stop"}]}).to_string();
    assert_eq!(feed(decoder.as_mut(), &data).expect("chunk").len(), 1);
}

#[test]
fn given_more_than_one_choice_or_an_index_above_zero_when_decoded_then_it_is_a_payload_error() {
    let two =
        json!({"choices": [{"index": 0, "delta": {}}, {"index": 1, "delta": {}}]}).to_string();
    assert!(fails(&two).text().contains("choices"));
    let second_only = json!({"choices": [{"index": 1, "delta": {"content": "x"}}]}).to_string();
    assert!(fails(&second_only).text().contains("index"));
    let same_twice =
        json!({"choices": [{"index": 0, "delta": {}}, {"index": 0, "delta": {}}]}).to_string();
    assert!(fails(&same_twice).text().contains("choices"));
}

#[test]
fn given_unrepresentable_delta_fields_when_decoded_then_it_is_a_payload_error_naming_them() {
    let audio = chunk(json!({"audio": {"data": "AAAA"}}), None);
    assert!(fails(&audio).text().contains("audio"));
    let function_call = chunk(
        json!({"function_call": {"name": "f", "arguments": ""}}),
        None,
    );
    assert!(fails(&function_call).text().contains("function_call"));
    let empty_audio = chunk(json!({"audio": {}}), None);
    assert!(fails(&empty_audio).text().contains("audio"));
}

#[test]
fn given_null_unrepresentable_delta_fields_when_decoded_then_they_are_read_as_absent() {
    let mut decoder = plain_decoder();
    let data = chunk(
        json!({"audio": null, "function_call": null, "content": "ok"}),
        None,
    );
    assert!(feed(decoder.as_mut(), &data).is_ok());
}

#[test]
fn given_a_role_of_any_type_when_decoded_then_it_is_metadata() {
    let mut decoder = plain_decoder();
    for role in [json!("assistant"), json!(5), json!({"a": 1}), json!(null)] {
        let data = chunk(json!({"role": role, "content": "x"}), None);
        assert!(feed(decoder.as_mut(), &data).is_ok());
    }
}

#[test]
fn given_a_stream_cut_at_every_byte_when_decoded_then_each_cut_has_a_defined_outcome() {
    let data = chunk(
        json!({"content": "Hello", "tool_calls": [{"index": 0, "id": "c", "function": {"name": "f", "arguments": "{}"}}]}),
        Some("tool_calls"),
    );
    for cut in 0..data.len() {
        let mut decoder = plain_decoder();
        let prefix = data.get(..cut).unwrap_or_default();
        let _ = decoder.payload(Payload::new(None, prefix));
        let _ = decoder.end();
    }
}
