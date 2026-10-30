use br_llm_clients_common_call::ProviderEvent;
use br_llm_messages::ResponseEnding;

use crate::test_decode::*;

fn stopped_with(delta_fields: &str) -> Vec<ProviderEvent> {
    let mut session = Session::started();
    session.ok(&format!(
        r#"{{"type":"message_delta","delta":{delta_fields}}}"#
    ));
    session.ok(r#"{"type":"message_stop"}"#)
}

#[test]
fn given_each_documented_stop_reason_when_decoded_then_it_maps_as_the_table_says() {
    let table = [
        ("end_turn", ResponseEnding::finished()),
        ("max_tokens", ResponseEnding::output_exhaustion()),
        ("stop_sequence", ResponseEnding::stop_sequence()),
        ("tool_use", ResponseEnding::tool_use()),
        ("refusal", ResponseEnding::refusal()),
        (
            "pause_turn",
            ResponseEnding::unclassified("pause_turn").expect("an ending"),
        ),
        (
            "model_context_window_exceeded",
            ResponseEnding::unclassified("model_context_window_exceeded").expect("an ending"),
        ),
        (
            "something_new",
            ResponseEnding::unclassified("something_new").expect("an ending"),
        ),
    ];
    for (api, mapped) in table {
        let events = stopped_with(&format!(r#"{{"stop_reason":"{api}"}}"#));
        assert_eq!(events, vec![stop(mapped), ProviderEvent::Complete], "{api}");
    }
}

#[test]
fn given_a_stop_reason_that_differs_by_case_or_space_when_decoded_then_it_is_compared_as_it_is() {
    for api in ["END_TURN", " end_turn", "end_turn ", "Tool_Use"] {
        let events = stopped_with(&format!(r#"{{"stop_reason":"{api}"}}"#));
        assert_eq!(
            events[0],
            stop(ResponseEnding::unclassified(api).expect("an ending")),
            "{api:?}"
        );
    }
}

#[test]
fn given_an_empty_or_non_text_stop_reason_when_decoded_then_it_is_a_payload_error() {
    for value in [r#""""#, "5", "true", "[]", r#"{"a":1}"#, "1.5"] {
        let mut session = Session::started();
        session.rejected(&format!(
            r#"{{"type":"message_delta","delta":{{"stop_reason":{value}}}}}"#
        ));
    }
}

#[test]
fn given_a_null_or_absent_stop_reason_when_decoded_then_the_recorded_one_stays() {
    let mut session = Session::started();
    session.ok(r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"}}"#);
    session.ok(r#"{"type":"message_delta","delta":{"stop_reason":null}}"#);
    session.ok(r#"{"type":"message_delta","delta":{}}"#);
    assert_eq!(
        session.ok(r#"{"type":"message_stop"}"#),
        vec![stop(ResponseEnding::finished()), ProviderEvent::Complete]
    );
}

#[test]
fn given_several_message_deltas_when_decoded_then_the_last_non_null_stop_reason_is_given_once() {
    let mut session = Session::started();
    session.ok(r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"}}"#);
    session.ok(r#"{"type":"message_delta","delta":{"stop_reason":"max_tokens"}}"#);
    session.ok(r#"{"type":"message_delta","delta":{"stop_reason":null}}"#);
    let events = session.ok(r#"{"type":"message_stop"}"#);
    assert_eq!(
        events,
        vec![
            stop(ResponseEnding::output_exhaustion()),
            ProviderEvent::Complete
        ]
    );
}

#[test]
fn given_a_message_delta_when_decoded_then_it_gives_no_stop_before_message_stop() {
    let mut session = Session::started();
    let events = session.ok(r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"}}"#);
    assert!(events.is_empty());
}

#[test]
fn given_a_message_stop_with_no_recorded_stop_reason_when_decoded_then_only_complete_is_given() {
    let mut session = Session::started();
    assert_eq!(
        session.ok(r#"{"type":"message_stop"}"#),
        vec![ProviderEvent::Complete]
    );
}

#[test]
fn given_a_message_delta_without_a_delta_object_when_decoded_then_it_is_a_payload_error() {
    for payload in [
        r#"{"type":"message_delta"}"#,
        r#"{"type":"message_delta","delta":null}"#,
        r#"{"type":"message_delta","delta":"end_turn"}"#,
        r#"{"type":"message_delta","delta":[]}"#,
        r#"{"type":"message_delta","usage":{"output_tokens":1}}"#,
    ] {
        Session::started().rejected(payload);
    }
}

#[test]
fn given_other_fields_of_the_delta_when_decoded_then_they_are_ignored() {
    let events =
        stopped_with(r#"{"stop_reason":"end_turn","stop_sequence":"X","container":{"id":"c"}}"#);
    assert_eq!(
        events,
        vec![stop(ResponseEnding::finished()), ProviderEvent::Complete]
    );
}

#[test]
fn given_a_stop_reason_in_message_start_when_decoded_then_it_is_not_recorded() {
    let mut session = Session::plain();
    session.ok(r#"{"type":"message_start","message":{"stop_reason":"end_turn"}}"#);
    assert_eq!(
        session.ok(r#"{"type":"message_stop"}"#),
        vec![ProviderEvent::Complete]
    );
}

#[test]
fn given_a_stop_sequence_stop_with_its_matched_sequence_when_decoded_then_the_ending_has_no_matched_sequence()
 {
    for delta in [
        r#"{"stop_reason":"stop_sequence","stop_sequence":"END"}"#,
        r#"{"stop_reason":"stop_sequence","stop_sequence":null}"#,
        r#"{"stop_reason":"stop_sequence"}"#,
        r#"{"stop_reason":"stop_sequence","stop_sequence":5,"stop_details":{"a":1}}"#,
    ] {
        let events = stopped_with(delta);
        assert_eq!(
            events,
            vec![
                stop(ResponseEnding::stop_sequence()),
                ProviderEvent::Complete
            ],
            "{delta}"
        );
        match &events[0] {
            ProviderEvent::Stop(ending) => assert_eq!(ending.matched_sequence(), None),
            other => panic!("not a stop: {other:?}"),
        }
    }
}
