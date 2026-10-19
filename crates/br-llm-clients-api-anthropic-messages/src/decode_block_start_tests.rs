use br_llm_clients_common_call::{ProviderBlock, ProviderEvent};

use crate::test_decode::*;

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
