use br_llm_clients_common_call::ProviderEvent;
use br_llm_messages::Usage;
use serde_json::json;

use crate::test_decoder::{feed, feed_err, plain_decoder};
use crate::test_events::completed;

fn usage_event(usage: serde_json::Value) -> Option<Usage> {
    let mut decoder = plain_decoder();
    feed(&mut decoder, &completed("m", usage))
        .into_iter()
        .find_map(|event| match event {
            ProviderEvent::Usage(usage) => Some(usage),
            _ => None,
        })
}

#[test]
fn given_a_full_usage_object_when_read_then_each_count_lands_in_its_field() {
    let usage = usage_event(json!({
        "input_tokens": 1200,
        "input_tokens_details": { "cached_tokens": 1024, "cache_write_tokens": 64 },
        "output_tokens": 300,
        "output_tokens_details": { "reasoning_tokens": 200 },
        "total_tokens": 1500
    }));
    assert_eq!(
        usage,
        Some(Usage {
            input_tokens: 1200,
            output_tokens: 300,
            cache_read_tokens: Some(1024),
            cache_write_tokens: Some(64),
            thinking_tokens: Some(200),
        })
    );
}

#[test]
fn given_a_cache_write_count_when_read_then_the_input_count_is_not_adjusted_for_it() {
    let usage = usage_event(json!({
        "input_tokens": 100,
        "input_tokens_details": { "cached_tokens": 10, "cache_write_tokens": 50 },
        "output_tokens": 5
    }))
    .expect("usage");
    assert_eq!(usage.input_tokens, 100);
    assert_eq!(usage.cache_write_tokens, Some(50));
}

#[test]
fn given_absent_detail_fields_when_read_then_they_are_absent_and_a_zero_is_a_zero() {
    let absent = usage_event(json!({ "input_tokens": 3, "output_tokens": 4 })).expect("usage");
    assert_eq!(
        (
            absent.cache_read_tokens,
            absent.cache_write_tokens,
            absent.thinking_tokens
        ),
        (None, None, None)
    );
    let zero = usage_event(json!({
        "input_tokens": 0, "output_tokens": 0,
        "input_tokens_details": { "cached_tokens": 0, "cache_write_tokens": 0 },
        "output_tokens_details": { "reasoning_tokens": 0 }
    }))
    .expect("usage");
    assert_eq!(
        zero,
        Usage {
            input_tokens: 0,
            output_tokens: 0,
            cache_read_tokens: Some(0),
            cache_write_tokens: Some(0),
            thinking_tokens: Some(0),
        }
    );
}

#[test]
fn given_a_usage_object_of_the_older_documented_shape_when_read_then_cache_write_is_absent() {
    let usage = usage_event(json!({
        "input_tokens": 37, "input_tokens_details": { "cached_tokens": 0 },
        "output_tokens": 11, "output_tokens_details": { "reasoning_tokens": 0 }, "total_tokens": 48
    }))
    .expect("usage");
    assert_eq!(usage.cache_write_tokens, None);
    assert_eq!(usage.cache_read_tokens, Some(0));
}

#[test]
fn given_total_tokens_that_disagree_when_read_then_it_is_ignored() {
    let usage =
        usage_event(json!({ "input_tokens": 1, "output_tokens": 2, "total_tokens": "many" }));
    assert!(usage.is_some());
}

#[test]
fn given_no_usage_or_a_null_usage_when_read_then_no_event_comes() {
    assert_eq!(usage_event(json!(null)), None);
    let mut decoder = plain_decoder();
    let mut event = completed("m", json!(null));
    event["response"]
        .as_object_mut()
        .expect("response")
        .remove("usage");
    let events = feed(&mut decoder, &event);
    assert!(
        events
            .iter()
            .all(|event| !matches!(event, ProviderEvent::Usage(_)))
    );
}

#[test]
fn given_a_usage_object_missing_one_of_the_two_main_counts_when_read_then_no_event_comes() {
    assert_eq!(usage_event(json!({ "input_tokens": 3 })), None);
    assert_eq!(usage_event(json!({ "output_tokens": 3 })), None);
    assert_eq!(usage_event(json!({})), None);
    assert_eq!(
        usage_event(json!({ "input_tokens": null, "output_tokens": 3 })),
        None
    );
}

#[test]
fn given_counts_that_are_not_whole_numbers_from_zero_when_read_then_it_is_a_payload_error() {
    let bad = [
        json!(-1),
        json!(1.5),
        json!(2.0),
        json!("7"),
        json!(true),
        json!([1]),
        json!(1e30),
    ];
    for value in bad {
        for place in 0..5 {
            let mut usage = json!({
                "input_tokens": 1,
                "input_tokens_details": { "cached_tokens": 1, "cache_write_tokens": 1 },
                "output_tokens": 1,
                "output_tokens_details": { "reasoning_tokens": 1 }
            });
            match place {
                0 => usage["input_tokens"] = value.clone(),
                1 => usage["output_tokens"] = value.clone(),
                2 => usage["input_tokens_details"]["cached_tokens"] = value.clone(),
                3 => usage["input_tokens_details"]["cache_write_tokens"] = value.clone(),
                _ => usage["output_tokens_details"]["reasoning_tokens"] = value.clone(),
            }
            let mut decoder = plain_decoder();
            let error = feed_err(&mut decoder, &completed("m", usage));
            assert!(
                error.text().starts_with("response.completed"),
                "{value} {place}"
            );
        }
    }
}

#[test]
fn given_a_usage_that_is_not_an_object_when_read_then_it_is_a_payload_error() {
    for usage in [json!(5), json!("x"), json!([1, 2])] {
        let mut decoder = plain_decoder();
        feed_err(&mut decoder, &completed("m", usage));
    }
}

#[test]
fn given_details_that_are_not_objects_when_read_then_it_is_a_payload_error() {
    for details in [json!(5), json!("x"), json!([])] {
        let mut decoder = plain_decoder();
        let usage =
            json!({ "input_tokens": 1, "output_tokens": 1, "input_tokens_details": details });
        feed_err(&mut decoder, &completed("m", usage));
    }
}

#[test]
fn given_the_largest_count_when_read_then_it_is_kept() {
    let usage =
        usage_event(json!({ "input_tokens": u64::MAX, "output_tokens": 0 })).expect("usage");
    assert_eq!(usage.input_tokens, u64::MAX);
}
