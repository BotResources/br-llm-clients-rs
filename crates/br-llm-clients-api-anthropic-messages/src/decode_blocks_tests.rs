use br_llm_clients_common_call::{ProviderBlock, ProviderEvent};

use crate::test_decode::*;

fn block_start(index: &str, block: &str) -> String {
    format!(r#"{{"type":"content_block_start","index":{index},"content_block":{block}}}"#)
}

fn delta(index: u64, delta: &str) -> String {
    format!(r#"{{"type":"content_block_delta","index":{index},"delta":{delta}}}"#)
}

fn stop_of(index: u64) -> String {
    format!(r#"{{"type":"content_block_stop","index":{index}}}"#)
}

#[test]
fn given_a_text_start_with_text_when_decoded_then_a_delta_follows_the_start() {
    let mut session = Session::started();
    let events = session.ok(&block_start("0", r#"{"type":"text","text":"Hi"}"#));
    assert_eq!(
        events,
        vec![start("0", ProviderBlock::Text), text_delta("0", "Hi")]
    );
}

#[test]
fn given_a_text_start_with_no_text_when_decoded_then_only_the_start_is_given() {
    for block in [
        r#"{"type":"text","text":""}"#,
        r#"{"type":"text"}"#,
        r#"{"type":"text","text":null}"#,
    ] {
        let mut session = Session::started();
        assert_eq!(
            session.ok(&block_start("0", block)),
            vec![start("0", ProviderBlock::Text)]
        );
    }
}

#[test]
fn given_a_text_start_whose_text_is_not_text_when_decoded_then_it_is_a_payload_error() {
    for block in [
        r#"{"type":"text","text":5}"#,
        r#"{"type":"text","text":["a"]}"#,
    ] {
        Session::started().rejected(&block_start("0", block));
    }
}

#[test]
fn given_the_structured_mode_when_a_text_block_starts_then_it_is_structured() {
    let mut session = Session::structured();
    session.ok(r#"{"type":"message_start","message":{}}"#);
    let events = session.ok(&block_start("0", r#"{"type":"text","text":"{"}"#));
    assert_eq!(
        events,
        vec![
            start("0", ProviderBlock::Structured),
            ProviderEvent::StructuredDelta {
                key: key("0"),
                json: "{".to_owned()
            },
        ]
    );
    let events = session.ok(&delta(0, r#"{"type":"text_delta","text":"}"}"#));
    assert_eq!(
        events,
        vec![ProviderEvent::StructuredDelta {
            key: key("0"),
            json: "}".to_owned()
        }]
    );
}

#[test]
fn given_a_thinking_start_when_decoded_then_non_empty_text_and_signature_are_given_in_that_order() {
    let mut session = Session::started();
    let events = session.ok(&block_start(
        "0",
        r#"{"type":"thinking","thinking":"hm","signature":"sig"}"#,
    ));
    assert_eq!(
        events,
        vec![
            start("0", ProviderBlock::Thinking),
            thinking_delta("0", "hm"),
            signature_delta("0", "sig"),
        ]
    );
}

#[test]
fn given_a_thinking_start_with_empty_fields_when_decoded_then_only_the_start_is_given() {
    for block in [
        r#"{"type":"thinking","thinking":"","signature":""}"#,
        r#"{"type":"thinking"}"#,
        r#"{"type":"thinking","thinking":null,"signature":null}"#,
    ] {
        let mut session = Session::started();
        assert_eq!(
            session.ok(&block_start("0", block)),
            vec![start("0", ProviderBlock::Thinking)]
        );
    }
}

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
fn given_a_redacted_thinking_start_without_usable_data_when_decoded_then_it_is_a_payload_error() {
    for block in [
        r#"{"type":"redacted_thinking"}"#,
        r#"{"type":"redacted_thinking","data":""}"#,
        r#"{"type":"redacted_thinking","data":null}"#,
        r#"{"type":"redacted_thinking","data":7}"#,
    ] {
        Session::started().rejected(&block_start("0", block));
    }
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
fn given_a_tool_use_start_when_decoded_then_name_and_id_are_given_as_text() {
    for block in [
        r#"{"type":"tool_use","id":"toolu_1","name":"lookup","input":{}}"#,
        r#"{"type":"tool_use","id":"toolu_1","name":"lookup"}"#,
    ] {
        let mut session = Session::started();
        assert_eq!(
            session.ok(&block_start("1", block)),
            vec![tool_start("1", "lookup", "toolu_1")]
        );
    }
}

#[test]
fn given_a_tool_use_start_without_a_usable_name_or_id_when_decoded_then_it_is_a_payload_error() {
    for block in [
        r#"{"type":"tool_use","name":"lookup"}"#,
        r#"{"type":"tool_use","id":"","name":"lookup"}"#,
        r#"{"type":"tool_use","id":5,"name":"lookup"}"#,
        r#"{"type":"tool_use","id":"toolu_1"}"#,
        r#"{"type":"tool_use","id":"toolu_1","name":""}"#,
        r#"{"type":"tool_use","id":"toolu_1","name":null}"#,
    ] {
        Session::started().rejected(&block_start("0", block));
    }
}

#[test]
fn given_a_tool_use_start_whose_input_is_not_empty_when_decoded_then_it_is_a_payload_error() {
    for input in ["null", r#"{"a":1}"#, "[]", "\"x\"", "0"] {
        let block = format!(r#"{{"type":"tool_use","id":"t","name":"n","input":{input}}}"#);
        Session::started().rejected(&block_start("0", &block));
    }
}

#[test]
fn given_a_block_type_that_is_not_accepted_when_decoded_then_it_is_a_payload_error() {
    for block in [
        r#"{"type":"server_tool_use","id":"s","name":"web_search","input":{}}"#,
        r#"{"type":"web_search_tool_result","tool_use_id":"s","content":[]}"#,
        r#"{"type":"fallback"}"#,
        r#"{"type":"mystery"}"#,
    ] {
        Session::started().rejected(&block_start("0", block));
    }
}

#[test]
fn given_a_malformed_block_start_when_decoded_then_it_is_a_payload_error() {
    let mut session = Session::started();
    for payload in [
        r#"{"type":"content_block_start","content_block":{"type":"text"}}"#,
        r#"{"type":"content_block_start","index":-1,"content_block":{"type":"text"}}"#,
        r#"{"type":"content_block_start","index":1.5,"content_block":{"type":"text"}}"#,
        r#"{"type":"content_block_start","index":"0","content_block":{"type":"text"}}"#,
        r#"{"type":"content_block_start","index":0}"#,
        r#"{"type":"content_block_start","index":0,"content_block":"text"}"#,
        r#"{"type":"content_block_start","index":0,"content_block":{}}"#,
        r#"{"type":"content_block_start","index":0,"content_block":{"type":3}}"#,
    ] {
        session.rejected(payload);
    }
}

#[test]
fn given_a_block_start_on_an_index_in_use_when_decoded_then_it_is_a_payload_error() {
    let mut session = Session::started();
    session.ok(&block_start("0", r#"{"type":"text"}"#));
    session.rejected(&block_start("0", r#"{"type":"text"}"#));
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

#[test]
fn given_fields_that_are_not_named_when_decoded_then_they_are_ignored() {
    let mut session = Session::started();
    let events = session.ok(
        r#"{"type":"content_block_start","index":0,"extra":1,"content_block":{"type":"text","text":"","citations":null,"more":true}}"#,
    );
    assert_eq!(events, vec![start("0", ProviderBlock::Text)]);
    let events = session.ok(
        r#"{"type":"content_block_delta","index":0,"x":[],"delta":{"type":"text_delta","text":"a","other":1}}"#,
    );
    assert_eq!(events, vec![text_delta("0", "a")]);
}
