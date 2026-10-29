use crate::test_decode::*;
use br_llm_clients_common_call::ProviderEvent;
use br_llm_messages::Usage;

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
fn given_no_thinking_tokens_in_any_payload_when_decoded_then_it_is_none() {
    let (mut session, _) = start_with(r#"{"input_tokens":1,"output_tokens":2}"#);
    let events = delta_usage(
        &mut session,
        r#"{"output_tokens":3,"output_tokens_details":{}}"#,
    );
    assert_eq!(only_usage(events).thinking_tokens(), None);
}

#[test]
fn given_a_thinking_tokens_of_zero_when_decoded_then_it_is_some_zero() {
    let (_, events) = start_with(
        r#"{"input_tokens":1,"output_tokens":2,"output_tokens_details":{"thinking_tokens":0}}"#,
    );
    assert_eq!(only_usage(events).thinking_tokens(), Some(0));
}

#[test]
fn given_a_null_details_or_a_null_thinking_tokens_when_decoded_then_it_counts_as_absent() {
    for details in ["null", r#"{"thinking_tokens":null}"#] {
        let (_, events) = start_with(&format!(
            r#"{{"input_tokens":1,"output_tokens":2,"output_tokens_details":{details}}}"#
        ));
        assert_eq!(only_usage(events).thinking_tokens(), None);
    }
}

#[test]
fn given_thinking_tokens_only_in_a_message_delta_when_decoded_then_it_is_some_after_it() {
    let (mut session, events) = start_with(r#"{"input_tokens":10,"output_tokens":1}"#);
    assert_eq!(only_usage(events).thinking_tokens(), None);
    let events = delta_usage(
        &mut session,
        r#"{"output_tokens":99,"output_tokens_details":{"thinking_tokens":41}}"#,
    );
    let usage = only_usage(events);
    assert_eq!(usage.thinking_tokens(), Some(41));
    assert_eq!(
        (usage.input_tokens(), usage.output_tokens()),
        (Some(10), Some(99))
    );
}

#[test]
fn given_thinking_tokens_in_message_start_when_decoded_then_it_is_read_there_too() {
    let (_, events) = start_with(
        r#"{"input_tokens":1,"output_tokens":2,"output_tokens_details":{"thinking_tokens":2}}"#,
    );
    assert_eq!(only_usage(events).thinking_tokens(), Some(2));
}

#[test]
fn given_a_later_payload_without_thinking_tokens_when_decoded_then_the_stored_one_stays() {
    let (mut session, _) = start_with(
        r#"{"input_tokens":1,"output_tokens":2,"output_tokens_details":{"thinking_tokens":2}}"#,
    );
    let events = delta_usage(&mut session, r#"{"output_tokens":9}"#);
    assert_eq!(only_usage(events).thinking_tokens(), Some(2));
    let events = delta_usage(
        &mut session,
        r#"{"output_tokens":10,"output_tokens_details":{"thinking_tokens":null}}"#,
    );
    assert_eq!(only_usage(events).thinking_tokens(), Some(2));
    let events = delta_usage(
        &mut session,
        r#"{"output_tokens":11,"output_tokens_details":null}"#,
    );
    assert_eq!(only_usage(events).thinking_tokens(), Some(2));
}

#[test]
fn given_a_later_thinking_tokens_when_decoded_then_it_replaces_the_stored_one() {
    let (mut session, _) = start_with(
        r#"{"input_tokens":1,"output_tokens":2,"output_tokens_details":{"thinking_tokens":2}}"#,
    );
    let events = delta_usage(
        &mut session,
        r#"{"output_tokens_details":{"thinking_tokens":7}}"#,
    );
    assert_eq!(only_usage(events).thinking_tokens(), Some(7));
}

#[test]
fn given_only_thinking_tokens_changing_after_completion_when_decoded_then_a_new_usage_is_given() {
    let (mut session, _) = start_with(r#"{"input_tokens":10,"output_tokens":5}"#);
    let events = delta_usage(
        &mut session,
        r#"{"output_tokens_details":{"thinking_tokens":3}}"#,
    );
    let usage = only_usage(events);
    assert_eq!(usage.thinking_tokens(), Some(3));
    assert_eq!(
        (usage.input_tokens(), usage.output_tokens()),
        (Some(10), Some(5))
    );
    assert!(
        delta_usage(
            &mut session,
            r#"{"output_tokens_details":{"thinking_tokens":3}}"#
        )
        .is_empty()
    );
}

#[test]
fn given_thinking_tokens_that_is_not_a_whole_number_from_zero_when_decoded_then_it_is_a_payload_error()
 {
    for value in ["-1", "1.5", "2.0", r#""7""#, "true", "[]", "{}", "1e3"] {
        Session::plain().rejected(&format!(
            r#"{{"type":"message_start","message":{{"usage":{{"output_tokens_details":{{"thinking_tokens":{value}}}}}}}}}"#
        ));
        Session::started().rejected(&format!(
            r#"{{"type":"message_delta","delta":{{}},"usage":{{"output_tokens_details":{{"thinking_tokens":{value}}}}}}}"#
        ));
    }
}

#[test]
fn given_output_tokens_details_that_is_not_an_object_when_decoded_then_it_is_a_payload_error() {
    for value in [r#""x""#, "5", "[]", "true"] {
        Session::plain().rejected(&format!(
            r#"{{"type":"message_start","message":{{"usage":{{"output_tokens_details":{value}}}}}}}"#
        ));
        Session::started().rejected(&format!(
            r#"{{"type":"message_delta","delta":{{}},"usage":{{"output_tokens_details":{value}}}}}"#
        ));
    }
}

#[test]
fn given_thinking_tokens_with_no_output_count_when_decoded_then_no_thinking_count_is_given() {
    let (mut session, events) =
        start_with(r#"{"input_tokens":3,"output_tokens_details":{"thinking_tokens":2}}"#);
    assert_eq!(only_usage(events), Usage::of_input_tokens(3));
    let events = delta_usage(&mut session, r#"{"output_tokens":9}"#);
    assert_eq!(only_usage(events), usage(3, 9).with_thinking_tokens(2));
}

#[test]
fn given_only_thinking_tokens_when_decoded_then_no_usage_is_given() {
    let (mut session, events) = start_with(r#"{"output_tokens_details":{"thinking_tokens":2}}"#);
    assert!(events.is_empty());
    let events = delta_usage(&mut session, r#"{"output_tokens":4}"#);
    assert_eq!(
        only_usage(events),
        Usage::of_output_tokens(4).with_thinking_tokens(2)
    );
}
