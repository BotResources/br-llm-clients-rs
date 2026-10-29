use br_llm_clients_common_call::{End, ErrorInfo, Payload, ProviderEvent};

use crate::test_decode::*;

const IN_MESSAGE_EVENTS: [&str; 5] = [
    r#"{"type":"content_block_start","index":0,"content_block":{"type":"text"}}"#,
    r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"x"}}"#,
    r#"{"type":"content_block_stop","index":0}"#,
    r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"}}"#,
    r#"{"type":"message_stop"}"#,
];

#[test]
fn given_a_message_start_with_a_model_and_usage_when_decoded_then_model_comes_first_then_usage() {
    let mut session = Session::plain();
    let events = session.ok(
        r#"{"type":"message_start","message":{"model":"claude-x","usage":{"input_tokens":4,"output_tokens":1}}}"#,
    );
    assert_eq!(
        events,
        vec![
            ProviderEvent::Model("claude-x".to_owned()),
            ProviderEvent::Usage(usage(4, 1)),
        ]
    );
}

#[test]
fn given_a_message_start_without_a_usable_model_when_decoded_then_no_model_is_given() {
    for message in [
        r#"{}"#,
        r#"{"model":""}"#,
        r#"{"model":5}"#,
        r#"{"model":null}"#,
    ] {
        let mut session = Session::plain();
        let events = session.ok(&format!(
            r#"{{"type":"message_start","message":{message}}}"#
        ));
        assert!(events.is_empty(), "{message}");
    }
}

#[test]
fn given_a_message_start_without_a_message_object_when_decoded_then_it_is_a_payload_error() {
    for payload in [
        r#"{"type":"message_start"}"#,
        r#"{"type":"message_start","message":null}"#,
        r#"{"type":"message_start","message":"m"}"#,
        r#"{"type":"message_start","message":[]}"#,
    ] {
        Session::plain().rejected(payload);
    }
}

#[test]
fn given_a_second_message_start_when_decoded_then_it_is_a_payload_error() {
    let mut session = Session::started();
    session.rejected(r#"{"type":"message_start","message":{}}"#);
}

#[test]
fn given_events_of_the_table_before_message_start_when_decoded_then_they_are_payload_errors() {
    for payload in IN_MESSAGE_EVENTS {
        Session::plain().rejected(payload);
    }
}

#[test]
fn given_a_ping_or_an_error_before_message_start_when_decoded_then_they_are_accepted() {
    let mut session = Session::plain();
    assert!(session.ok(r#"{"type":"ping"}"#).is_empty());
    let events =
        session.ok(r#"{"type":"error","error":{"type":"overloaded_error","message":"x"}}"#);
    assert!(matches!(events.as_slice(), [ProviderEvent::Error(_)]));
}

#[test]
fn given_an_event_type_that_is_not_in_the_table_when_decoded_then_it_is_ignored_in_every_state() {
    let mut session = Session::plain();
    assert!(
        session
            .ok(r#"{"type":"future_event","payload":{"a":1}}"#)
            .is_empty()
    );
    session.ok(r#"{"type":"message_start","message":{}}"#);
    assert!(
        session
            .ok(r#"{"type":"future_event","index":0}"#)
            .is_empty()
    );
    assert!(
        session
            .ok(r#"{"type":"content_block_start_v2"}"#)
            .is_empty()
    );
}

#[test]
fn given_an_event_name_that_differs_from_the_type_when_decoded_then_it_is_a_payload_error() {
    let mut decoder = decoder(false);
    let payload = Payload::new(Some("message_start".to_owned()), r#"{"type":"ping"}"#);
    assert!(decoder.payload(payload).is_err());
    let payload = Payload::new(Some("ping".to_owned()), r#"{"type":"future_event"}"#);
    assert!(decoder.payload(payload).is_err());
}

#[test]
fn given_a_payload_without_an_event_name_when_decoded_then_the_type_alone_decides() {
    let mut decoder = decoder(false);
    let payload = Payload::new(None, r#"{"type":"message_start","message":{"model":"m"}}"#);
    assert_eq!(
        decoder.payload(payload),
        Ok(vec![ProviderEvent::Model("m".to_owned())])
    );
}

#[test]
fn given_data_that_is_not_an_object_with_a_text_type_when_decoded_then_it_is_a_payload_error() {
    let mut decoder = decoder(false);
    for data in [
        "",
        "not json",
        "[]",
        "\"ping\"",
        "null",
        "5",
        "{}",
        r#"{"type":5}"#,
        r#"{"type":null}"#,
    ] {
        let payload = Payload::new(None, data);
        assert!(decoder.payload(payload).is_err(), "{data:?}");
    }
}

#[test]
fn given_a_payload_error_when_read_then_its_text_holds_no_payload_content() {
    let mut decoder = decoder(false);
    let error = decoder
        .payload(Payload::new(None, "secret-user-content"))
        .expect_err("an error");
    assert!(!error.text().contains("secret-user-content"));
}

#[test]
fn given_an_event_named_error_when_decoded_then_it_gives_an_error_whatever_the_data_holds() {
    let mut decoder = decoder(false);
    let events = decoder
        .payload(Payload::new(Some("error".to_owned()), "plain words"))
        .expect("an error event");
    assert_eq!(
        events,
        vec![ProviderEvent::Error(ErrorInfo::new("plain words"))]
    );
    let data = r#"{"type":"ping"}"#;
    let events = decoder
        .payload(Payload::new(Some("error".to_owned()), data))
        .expect("an error event");
    assert_eq!(events, vec![ProviderEvent::Error(ErrorInfo::new(data))]);
}

#[test]
fn given_data_of_type_error_when_decoded_then_the_code_is_the_error_type_and_the_text_is_as_received()
 {
    let data = r#"{"type":"error","error":{"type":"overloaded_error","message":"Overloaded"},"request_id":"req_1"}"#;
    let mut session = Session::plain();
    assert_eq!(
        session.ok(data),
        vec![ProviderEvent::Error(
            ErrorInfo::new(data).with_code("overloaded_error")
        )]
    );
}

#[test]
fn given_data_of_type_error_with_no_event_name_or_another_one_when_decoded_then_it_is_the_error_event()
 {
    let data = r#"{"type":"error","error":{"type":"overloaded_error","message":"x"}}"#;
    let expected = vec![ProviderEvent::Error(
        ErrorInfo::new(data).with_code("overloaded_error"),
    )];
    for event in [
        None,
        Some("message_start".to_owned()),
        Some("ping".to_owned()),
    ] {
        let mut decoder = decoder(false);
        let events = decoder
            .payload(Payload::new(event.clone(), data))
            .expect("an error event");
        assert_eq!(events, expected, "{event:?}");
    }
}

#[test]
fn given_data_of_another_type_with_no_event_name_when_decoded_then_it_is_not_an_error_event() {
    let mut decoder = decoder(false);
    let events = decoder
        .payload(Payload::new(
            None,
            r#"{"type":"message_start","message":{"model":"m"}}"#,
        ))
        .expect("a message start");
    assert_eq!(events, vec![ProviderEvent::Model("m".to_owned())]);
}

#[test]
fn given_an_error_event_with_no_text_error_type_when_decoded_then_it_has_no_code() {
    for data in [
        r#"{"type":"error"}"#,
        r#"{"type":"error","error":{}}"#,
        r#"{"type":"error","error":{"type":5}}"#,
        r#"{"type":"error","error":"boom"}"#,
    ] {
        let mut session = Session::plain();
        assert_eq!(
            session.ok(data),
            vec![ProviderEvent::Error(ErrorInfo::new(data))],
            "{data}"
        );
    }
}

#[test]
fn given_an_error_event_when_decoded_then_it_has_no_status_and_no_delay() {
    let mut session = Session::started();
    let events = session.ok(r#"{"type":"error","error":{"type":"api_error","message":"x"}}"#);
    let [ProviderEvent::Error(info)] = events.as_slice() else {
        panic!("expected one error event");
    };
    assert_eq!(info.status(), None);
    assert_eq!(info.retry_after(), None);
}

#[test]
fn given_any_state_when_the_stream_ends_without_message_stop_then_the_marker_is_missing() {
    let mut fresh = Session::plain();
    assert_eq!(fresh.end(), Ok(End::MarkerMissing));
    let mut midway = Session::started();
    midway.ok(IN_MESSAGE_EVENTS[0]);
    midway.ok(IN_MESSAGE_EVENTS[1]);
    assert_eq!(midway.end(), Ok(End::MarkerMissing));
    let mut complete = Session::started();
    complete.ok(IN_MESSAGE_EVENTS[3]);
    complete.ok(IN_MESSAGE_EVENTS[4]);
    assert_eq!(complete.end(), Ok(End::MarkerMissing));
}

#[test]
fn given_a_ping_in_a_message_when_decoded_then_it_gives_nothing() {
    let mut session = Session::started();
    assert!(session.ok(r#"{"type":"ping"}"#).is_empty());
}

#[test]
fn given_an_extra_field_in_the_message_when_decoded_then_it_is_ignored() {
    let mut session = Session::plain();
    let events = session.ok(
        r#"{"type":"message_start","extra":1,"message":{"role":"assistant","content":[],"stop_reason":null,"stop_sequence":null,"container":null,"stop_details":null}}"#,
    );
    assert!(events.is_empty());
}
