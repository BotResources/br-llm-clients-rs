use br_llm_clients_common_call::{ProviderBlock, ProviderEvent};

use crate::test_decode::*;

#[test]
fn given_a_redacted_thinking_start_when_decoded_then_the_data_is_given_and_the_block_closes_at_its_stop()
 {
    let mut session = Session::started();
    let events = session.ok(&block_start(
        "3",
        r#"{"type":"redacted_thinking","data":"abc"}"#,
    ));
    assert_eq!(
        events,
        vec![
            start("3", ProviderBlock::RedactedThinking),
            ProviderEvent::RedactedThinkingData {
                key: key("3"),
                data: "abc".to_owned()
            },
        ]
    );
    assert_eq!(session.ok(&stop_of(3)), vec![end_of("3")]);
}

#[test]
fn given_a_redacted_thinking_block_when_a_delta_arrives_then_it_is_a_payload_error() {
    let mut session = Session::started();
    session.ok(&block_start(
        "0",
        r#"{"type":"redacted_thinking","data":"abc"}"#,
    ));
    for kind in [
        r#"{"type":"text_delta","text":"x"}"#,
        r#"{"type":"thinking_delta","thinking":"x"}"#,
        r#"{"type":"signature_delta","signature":"x"}"#,
        r#"{"type":"input_json_delta","partial_json":"x"}"#,
    ] {
        session.rejected(&delta(0, kind));
    }
}

#[test]
fn given_a_closed_text_index_when_a_block_starts_there_again_then_it_is_accepted() {
    let mut session = Session::started();
    session.ok(&block_start("0", r#"{"type":"text"}"#));
    session.ok(&stop_of(0));
    assert_eq!(
        session.ok(&block_start("0", r#"{"type":"text"}"#)),
        vec![start("0", ProviderBlock::Text)]
    );
}

#[test]
fn given_the_index_of_a_block_when_decoded_then_the_key_is_its_decimal_text() {
    let mut session = Session::started();
    let events = session.ok(&block_start("12", r#"{"type":"text"}"#));
    assert_eq!(events, vec![start("12", ProviderBlock::Text)]);
    assert_eq!(
        session.ok(&delta(12, r#"{"type":"text_delta","text":"x"}"#)),
        vec![text_delta("12", "x")]
    );
    assert_eq!(session.ok(&stop_of(12)), vec![end_of("12")]);
}

#[test]
fn given_each_delta_kind_when_decoded_then_the_matching_event_is_given() {
    let mut session = Session::started();
    session.ok(&block_start("0", r#"{"type":"text"}"#));
    session.ok(&block_start("1", r#"{"type":"thinking"}"#));
    session.ok(&block_start(
        "2",
        r#"{"type":"tool_use","id":"t","name":"n","input":{}}"#,
    ));
    assert_eq!(
        session.ok(&delta(0, r#"{"type":"text_delta","text":"a"}"#)),
        vec![text_delta("0", "a")]
    );
    assert_eq!(
        session.ok(&delta(1, r#"{"type":"thinking_delta","thinking":"b"}"#)),
        vec![thinking_delta("1", "b")]
    );
    assert_eq!(
        session.ok(&delta(1, r#"{"type":"signature_delta","signature":"c"}"#)),
        vec![signature_delta("1", "c")]
    );
    assert_eq!(
        session.ok(&delta(
            2,
            r#"{"type":"input_json_delta","partial_json":"{\"a\""}"#
        )),
        vec![args_delta("2", "{\"a\"")]
    );
}

#[test]
fn given_a_delta_with_an_empty_value_when_decoded_then_no_event_is_given() {
    let mut session = Session::started();
    session.ok(&block_start("0", r#"{"type":"text"}"#));
    session.ok(&block_start("1", r#"{"type":"thinking"}"#));
    session.ok(&block_start(
        "2",
        r#"{"type":"tool_use","id":"t","name":"n","input":{}}"#,
    ));
    for (index, payload) in [
        (0, r#"{"type":"text_delta","text":""}"#),
        (1, r#"{"type":"thinking_delta","thinking":""}"#),
        (1, r#"{"type":"signature_delta","signature":""}"#),
        (2, r#"{"type":"input_json_delta","partial_json":""}"#),
    ] {
        assert!(session.ok(&delta(index, payload)).is_empty());
    }
}

#[test]
fn given_a_delta_that_does_not_fit_its_block_when_decoded_then_it_is_a_payload_error() {
    let mut session = Session::started();
    session.ok(&block_start("0", r#"{"type":"text"}"#));
    session.ok(&block_start("1", r#"{"type":"thinking"}"#));
    session.ok(&block_start(
        "2",
        r#"{"type":"tool_use","id":"t","name":"n","input":{}}"#,
    ));
    session.rejected(&delta(0, r#"{"type":"thinking_delta","thinking":"x"}"#));
    session.rejected(&delta(
        0,
        r#"{"type":"input_json_delta","partial_json":"x"}"#,
    ));
    session.rejected(&delta(1, r#"{"type":"text_delta","text":"x"}"#));
    session.rejected(&delta(2, r#"{"type":"text_delta","text":"x"}"#));
    session.rejected(&delta(2, r#"{"type":"signature_delta","signature":"x"}"#));
}

#[test]
fn given_a_delta_of_an_unknown_type_when_decoded_then_it_is_a_payload_error() {
    let mut session = Session::started();
    session.ok(&block_start("0", r#"{"type":"text"}"#));
    session.rejected(&delta(0, r#"{"type":"citations_delta","citation":{}}"#));
    session.rejected(&delta(0, r#"{"type":"mystery_delta","text":"x"}"#));
}

#[test]
fn given_a_malformed_delta_when_decoded_then_it_is_a_payload_error() {
    let mut session = Session::started();
    session.ok(&block_start("0", r#"{"type":"text"}"#));
    for payload in [
        r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"x"}}"#,
        r#"{"type":"content_block_delta","index":"0","delta":{"type":"text_delta","text":"x"}}"#,
        r#"{"type":"content_block_delta","index":0}"#,
        r#"{"type":"content_block_delta","index":0,"delta":"x"}"#,
        r#"{"type":"content_block_delta","index":0,"delta":{}}"#,
        r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta"}}"#,
        r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":null}}"#,
        r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":4}}"#,
    ] {
        session.rejected(payload);
    }
}

#[test]
fn given_a_delta_for_a_block_that_is_not_open_when_decoded_then_it_is_a_payload_error() {
    let mut session = Session::started();
    session.rejected(&delta(0, r#"{"type":"text_delta","text":"x"}"#));
    session.ok(&block_start("0", r#"{"type":"text"}"#));
    session.ok(&stop_of(0));
    session.rejected(&delta(0, r#"{"type":"text_delta","text":"x"}"#));
}

#[test]
fn given_a_stop_for_a_block_that_is_not_open_when_decoded_then_it_is_a_payload_error() {
    let mut session = Session::started();
    session.rejected(&stop_of(0));
    session.ok(&block_start("0", r#"{"type":"text"}"#));
    session.ok(&stop_of(0));
    session.rejected(&stop_of(0));
    session.rejected(r#"{"type":"content_block_stop"}"#);
}

#[test]
fn given_a_block_of_each_closed_kind_when_it_stops_then_a_block_end_is_given() {
    let mut session = Session::started();
    session.ok(&block_start("0", r#"{"type":"text"}"#));
    session.ok(&block_start("1", r#"{"type":"thinking"}"#));
    session.ok(&block_start(
        "2",
        r#"{"type":"tool_use","id":"t","name":"n","input":{}}"#,
    ));
    session.ok(&delta(
        2,
        r#"{"type":"input_json_delta","partial_json":"{}"}"#,
    ));
    assert_eq!(session.ok(&stop_of(0)), vec![end_of("0")]);
    assert_eq!(session.ok(&stop_of(1)), vec![end_of("1")]);
    assert_eq!(session.ok(&stop_of(2)), vec![end_of("2")]);
}

#[test]
fn given_blocks_open_at_the_same_time_when_decoded_then_events_keep_their_own_keys() {
    let mut session = Session::started();
    session.ok(&block_start("0", r#"{"type":"text"}"#));
    session.ok(&block_start("1", r#"{"type":"text"}"#));
    assert_eq!(
        session.ok(&delta(1, r#"{"type":"text_delta","text":"b"}"#)),
        vec![text_delta("1", "b")]
    );
    assert_eq!(
        session.ok(&delta(0, r#"{"type":"text_delta","text":"a"}"#)),
        vec![text_delta("0", "a")]
    );
    assert_eq!(session.ok(&stop_of(1)), vec![end_of("1")]);
    assert_eq!(session.ok(&stop_of(0)), vec![end_of("0")]);
}
