use br_llm_messages::Usage;
use serde_json::json;

use crate::decode_usage_tests::{one_usage, usages, with_cache_field};
use crate::test_support::{dialect_for, feed, options, usage_chunk};
use crate::{Dialect, OutputMode, ReasoningLevel};

#[test]
fn given_a_cache_count_without_the_prompt_count_when_decoded_then_the_cache_count_is_not_given() {
    let usage = json!({
        "completion_tokens": 5,
        "prompt_tokens_details": {"cached_tokens": 2, "cache_write_tokens": 1}
    });
    assert_eq!(one_usage(usage), Some(Usage::of_output_tokens(5)));
    let mut decoder = with_cache_field();
    let found = usages(
        decoder.as_mut(),
        &usage_chunk(json!({"completion_tokens": 5, "cached_prompt_tokens": 4})),
    );
    assert_eq!(found, [Usage::of_output_tokens(5)]);
}

#[test]
fn given_no_cache_field_in_the_options_when_a_host_sends_one_then_it_is_metadata() {
    let usage = json!({"prompt_tokens": 5, "completion_tokens": 1, "cached_prompt_tokens": 4});
    assert_eq!(
        one_usage(usage).and_then(|usage| usage.cache_read_input_tokens()),
        None
    );
}

#[test]
fn given_a_cache_field_when_the_details_hold_no_cached_tokens_then_the_field_is_the_source() {
    let mut decoder = with_cache_field();
    let usage = json!({"prompt_tokens": 5, "completion_tokens": 1, "cached_prompt_tokens": 4});
    let found = usages(decoder.as_mut(), &usage_chunk(usage));
    assert_eq!(
        found
            .first()
            .and_then(|usage| usage.cache_read_input_tokens()),
        Some(4)
    );
}

#[test]
fn given_a_cache_field_when_the_details_hold_cached_tokens_then_the_details_win() {
    let mut decoder = with_cache_field();
    let usage = json!({
        "prompt_tokens": 5, "completion_tokens": 1, "cached_prompt_tokens": 4,
        "prompt_tokens_details": {"cached_tokens": 2}
    });
    let found = usages(decoder.as_mut(), &usage_chunk(usage));
    assert_eq!(
        found
            .first()
            .and_then(|usage| usage.cache_read_input_tokens()),
        Some(2)
    );
}

#[test]
fn given_a_cache_field_when_the_details_hold_zero_cached_tokens_then_zero_wins_over_the_field() {
    let mut decoder = with_cache_field();
    let usage = json!({
        "prompt_tokens": 5, "completion_tokens": 1, "cached_prompt_tokens": 4,
        "prompt_tokens_details": {"cached_tokens": 0}
    });
    let found = usages(decoder.as_mut(), &usage_chunk(usage));
    assert_eq!(
        found
            .first()
            .and_then(|usage| usage.cache_read_input_tokens()),
        Some(0)
    );
}

#[test]
fn given_a_cache_field_when_the_details_cached_tokens_is_null_then_the_field_is_the_source() {
    let mut decoder = with_cache_field();
    let usage = json!({
        "prompt_tokens": 5, "completion_tokens": 1, "cached_prompt_tokens": 4,
        "prompt_tokens_details": {"cached_tokens": null}
    });
    let found = usages(decoder.as_mut(), &usage_chunk(usage));
    assert_eq!(
        found
            .first()
            .and_then(|usage| usage.cache_read_input_tokens()),
        Some(4)
    );
}

#[test]
fn given_a_cache_field_when_neither_source_reports_then_the_cache_read_is_absent() {
    let mut decoder = with_cache_field();
    let found = usages(
        decoder.as_mut(),
        &usage_chunk(json!({"prompt_tokens": 5, "completion_tokens": 1})),
    );
    assert_eq!(
        found
            .first()
            .and_then(|usage| usage.cache_read_input_tokens()),
        None
    );
}

#[test]
fn given_a_top_level_cached_tokens_member_when_it_is_the_cache_field_then_it_is_the_cache_read() {
    let configured = options().with_cache_read_field("cached_tokens");
    let mut decoder = dialect_for(ReasoningLevel::Off, configured).decoder(&OutputMode::Plain);
    let found = usages(
        decoder.as_mut(),
        &usage_chunk(json!({"prompt_tokens": 50, "completion_tokens": 3, "cached_tokens": 32})),
    );
    assert_eq!(
        found,
        [Usage::of_input_tokens(50)
            .with_output_tokens(3)
            .with_cache_read_input_tokens(32)]
    );
}

#[test]
fn given_a_bad_cache_field_count_when_the_field_is_the_source_then_it_is_a_payload_error() {
    let mut decoder = with_cache_field();
    let usage = json!({"prompt_tokens": 1, "completion_tokens": 1, "cached_prompt_tokens": "x"});
    let error = feed(decoder.as_mut(), &usage_chunk(usage)).expect_err("bad");
    assert!(error.text().contains("cached_prompt_tokens"));
}
