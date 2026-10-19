use br_llm_clients_common_call::ProviderEvent;
use br_llm_messages::Usage;

use crate::test_decode::*;

fn start_with(usage: &str) -> (Session, Vec<ProviderEvent>) {
    let mut session = Session::plain();
    let events = session.ok(&format!(
        r#"{{"type":"message_start","message":{{"usage":{usage}}}}}"#
    ));
    (session, events)
}

fn delta_usage(session: &mut Session, usage: &str) -> Vec<ProviderEvent> {
    session.ok(&format!(
        r#"{{"type":"message_delta","delta":{{}},"usage":{usage}}}"#
    ))
}

fn only_usage(events: Vec<ProviderEvent>) -> Usage {
    match events.as_slice() {
        [ProviderEvent::Usage(usage)] => usage.clone(),
        other => panic!("expected one usage event, got {other:?}"),
    }
}

#[test]
fn given_input_and_output_in_message_start_when_decoded_then_one_usage_is_given() {
    let (_, events) = start_with(r#"{"input_tokens":10,"output_tokens":2}"#);
    assert_eq!(only_usage(events), usage(10, 2));
}

#[test]
fn given_the_three_input_fields_when_decoded_then_input_is_their_sum_and_cache_fields_are_kept() {
    let (_, events) = start_with(
        r#"{"input_tokens":10,"cache_read_input_tokens":5,"cache_creation_input_tokens":3,"output_tokens":2}"#,
    );
    assert_eq!(
        only_usage(events),
        Usage {
            input_tokens: 18,
            output_tokens: 2,
            cache_read_tokens: Some(5),
            cache_write_tokens: Some(3),
            thinking_tokens: None,
        }
    );
}

#[test]
fn given_a_cache_field_never_present_when_decoded_then_it_is_none_and_a_zero_is_some_zero() {
    let (_, events) = start_with(r#"{"input_tokens":1,"output_tokens":1}"#);
    let usage = only_usage(events);
    assert_eq!(usage.cache_read_tokens, None);
    assert_eq!(usage.cache_write_tokens, None);
    let (_, events) = start_with(
        r#"{"input_tokens":1,"output_tokens":1,"cache_read_input_tokens":0,"cache_creation_input_tokens":0}"#,
    );
    let usage = only_usage(events);
    assert_eq!(usage.cache_read_tokens, Some(0));
    assert_eq!(usage.cache_write_tokens, Some(0));
}

#[test]
fn given_a_usage_without_output_when_decoded_then_none_is_given_until_both_have_been_present() {
    let (mut session, events) = start_with(r#"{"input_tokens":10}"#);
    assert!(events.is_empty());
    assert_eq!(
        only_usage(delta_usage(&mut session, r#"{"output_tokens":7}"#)),
        usage(10, 7)
    );
}

#[test]
fn given_a_usage_without_input_when_decoded_then_none_is_given_until_both_have_been_present() {
    let (mut session, events) = start_with(r#"{"output_tokens":4}"#);
    assert!(events.is_empty());
    assert_eq!(
        only_usage(delta_usage(&mut session, r#"{"input_tokens":9}"#)),
        usage(9, 4)
    );
}

#[test]
fn given_cumulative_counts_when_a_message_delta_brings_numbers_then_they_replace_the_stored_ones() {
    let (mut session, _) = start_with(r#"{"input_tokens":10,"output_tokens":1}"#);
    assert_eq!(
        only_usage(delta_usage(&mut session, r#"{"output_tokens":15}"#)),
        usage(10, 15)
    );
    assert_eq!(
        only_usage(delta_usage(
            &mut session,
            r#"{"input_tokens":12,"output_tokens":20}"#
        )),
        usage(12, 20)
    );
}

#[test]
fn given_a_null_or_absent_number_when_decoded_then_the_stored_one_stays() {
    let (mut session, _) = start_with(r#"{"input_tokens":10,"output_tokens":1}"#);
    let events = delta_usage(&mut session, r#"{"input_tokens":null,"output_tokens":6}"#);
    assert_eq!(only_usage(events), usage(10, 6));
}

#[test]
fn given_a_usage_that_changes_nothing_when_decoded_then_no_event_is_given() {
    let (mut session, _) = start_with(r#"{"input_tokens":10,"output_tokens":5}"#);
    assert!(delta_usage(&mut session, r#"{"output_tokens":5}"#).is_empty());
    assert!(delta_usage(&mut session, r#"{}"#).is_empty());
    assert!(delta_usage(&mut session, "null").is_empty());
    assert!(
        delta_usage(
            &mut session,
            r#"{"server_tool_use":{"web_search_requests":2}}"#
        )
        .is_empty()
    );
    assert!(
        session
            .ok(r#"{"type":"message_delta","delta":{}}"#)
            .is_empty()
    );
}

#[test]
fn given_only_a_cache_field_changing_after_completion_when_decoded_then_a_new_usage_is_given() {
    let (mut session, _) = start_with(r#"{"input_tokens":10,"output_tokens":5}"#);
    let events = delta_usage(&mut session, r#"{"cache_read_input_tokens":4}"#);
    let usage = only_usage(events);
    assert_eq!(usage.input_tokens, 14);
    assert_eq!(usage.cache_read_tokens, Some(4));
}

#[test]
fn given_a_message_delta_usage_when_message_start_had_none_then_it_can_complete_the_record() {
    let mut session = Session::started();
    let events = delta_usage(&mut session, r#"{"input_tokens":3,"output_tokens":8}"#);
    assert_eq!(only_usage(events), usage(3, 8));
}

#[test]
fn given_other_usage_fields_when_decoded_then_they_are_ignored() {
    let (_, events) = start_with(
        r#"{"input_tokens":1,"output_tokens":2,"cache_creation":{"ephemeral_5m_input_tokens":0},"service_tier":"standard","server_tool_use":{"web_search_requests":0},"thinking_tokens":9}"#,
    );
    let usage = only_usage(events);
    assert_eq!(usage.thinking_tokens, None);
    assert_eq!(usage.cache_read_tokens, None);
    assert_eq!((usage.input_tokens, usage.output_tokens), (1, 2));
}

#[test]
fn given_a_number_that_is_not_a_non_negative_integer_when_decoded_then_it_is_a_payload_error() {
    for field in [
        "input_tokens",
        "output_tokens",
        "cache_read_input_tokens",
        "cache_creation_input_tokens",
    ] {
        for value in ["-1", "1.5", "2.0", r#""7""#, "true", "[]", "{}", "1e3"] {
            let mut session = Session::plain();
            session.rejected(&format!(
                r#"{{"type":"message_start","message":{{"usage":{{"{field}":{value}}}}}}}"#
            ));
            let mut session = Session::started();
            session.rejected(&format!(
                r#"{{"type":"message_delta","delta":{{}},"usage":{{"{field}":{value}}}}}"#
            ));
        }
    }
}

#[test]
fn given_a_usage_that_is_not_an_object_when_decoded_then_it_is_a_payload_error() {
    for value in [r#""x""#, "5", "[]", "true"] {
        Session::plain().rejected(&format!(
            r#"{{"type":"message_start","message":{{"usage":{value}}}}}"#
        ));
        Session::started().rejected(&format!(
            r#"{{"type":"message_delta","delta":{{}},"usage":{value}}}"#
        ));
    }
}

#[test]
fn given_a_sum_that_overflows_when_decoded_then_it_is_a_payload_error() {
    let mut session = Session::plain();
    session.rejected(&format!(
        r#"{{"type":"message_start","message":{{"usage":{{"input_tokens":{},"cache_read_input_tokens":1,"output_tokens":1}}}}}}"#,
        u64::MAX
    ));
}

#[test]
fn given_the_largest_exact_sum_when_decoded_then_it_is_accepted() {
    let (_, events) = start_with(&format!(
        r#"{{"input_tokens":{},"cache_read_input_tokens":1,"output_tokens":1}}"#,
        u64::MAX - 1
    ));
    assert_eq!(only_usage(events).input_tokens, u64::MAX);
}

#[test]
fn given_a_stream_with_no_usage_when_decoded_then_no_usage_event_is_given() {
    let events = decode_stream(false, crate::test_streams::THINKING);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, ProviderEvent::Usage(_)))
    );
}
