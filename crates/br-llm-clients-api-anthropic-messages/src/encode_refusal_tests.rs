use br_llm_clients_common_call::Dialect;
use br_llm_clients_core::{ClientError, OutputMode, ParameterKind};
use br_llm_messages::AssistantBlock;
use serde_json::json;

use crate::test_fixtures::*;

fn reason_of(error: ClientError) -> String {
    match error {
        ClientError::InvalidMessages { reason } => {
            assert!(!reason.is_empty());
            reason
        }
        other => panic!("expected InvalidMessages, got {other:?}"),
    }
}

fn refused(messages: Vec<br_llm_messages::WireMessage>) -> String {
    reason_of(refusal_of(&dialect(), &request(messages)))
}

#[test]
fn given_a_last_assistant_message_when_encoded_then_it_is_refused_as_a_prefill() {
    refused(vec![user("a"), assistant_text("b")]);
}

#[test]
fn given_a_last_assistant_message_after_a_user_and_a_tool_result_when_encoded_then_it_is_refused() {
    refused(vec![
        user("go"),
        assistant(vec![tool_call("t1", "lookup", json!({}))]),
        user_blocks(vec![tool_result("t1", "lookup", &["r"], false)]),
        assistant_text("done"),
    ]);
}

#[test]
fn given_a_user_message_with_no_block_when_encoded_then_it_is_refused() {
    refused(vec![user_blocks(vec![])]);
    refused(vec![user("a"), assistant_text("b"), user_blocks(vec![])]);
}

#[test]
fn given_an_empty_relay_after_an_assistant_message_when_encoded_then_the_empty_user_message_is_refused()
 {
    refused(vec![user("a"), assistant_text("b"), relay(&[])]);
}

#[test]
fn given_an_empty_user_message_followed_by_a_relay_when_encoded_then_the_relay_fills_it() {
    let request = request(vec![user_blocks(vec![]), relay(&["x"])]);
    let body = body_of(&dialect(), &request);
    assert_eq!(
        body["messages"],
        json!([{"role": "user", "content": [{"type": "text", "text": "x"}]}])
    );
}

#[test]
fn given_an_assistant_message_with_no_block_when_encoded_then_it_is_refused() {
    refused(vec![user("a"), assistant(vec![]), user("b")]);
}

#[test]
fn given_an_assistant_message_of_earlier_thinking_only_when_encoded_then_nothing_can_be_sent_and_it_is_refused()
 {
    refused(vec![
        user("a"),
        assistant(vec![thinking("hm", Some("sig")), redacted("data")]),
        user("b"),
    ]);
}

#[test]
fn given_tool_arguments_that_are_not_an_object_when_encoded_then_it_is_refused_naming_the_id() {
    for arguments in [
        json!([1]),
        json!("text"),
        json!(null),
        json!(7),
        json!(true),
    ] {
        let reason = refused(vec![
            user("go"),
            assistant(vec![tool_call("call-9", "lookup", arguments)]),
            user_blocks(vec![tool_result("call-9", "lookup", &["r"], false)]),
        ]);
        assert!(reason.contains("call-9"));
    }
}

#[test]
fn given_a_thinking_without_signature_in_the_turn_in_progress_when_encoded_then_it_is_refused() {
    refused(vec![
        user("go"),
        assistant(vec![
            thinking("hm", None),
            tool_call("t1", "lookup", json!({})),
        ]),
        user_blocks(vec![tool_result("t1", "lookup", &["r"], false)]),
    ]);
}

#[test]
fn given_a_refusal_when_encoded_then_the_reason_holds_no_content_of_the_prompt() {
    let reason = refused(vec![
        user("a very private question"),
        assistant(vec![AssistantBlock::Text {
            text: text("a very private answer"),
        }]),
    ]);
    assert!(!reason.contains("private"));
}

#[test]
fn given_a_structured_output_and_a_declared_tool_when_encoded_then_it_is_refused() {
    let request = request_with(
        None,
        vec![user("a")],
        vec![tool("lookup", "", vec![])],
        structured(None, vec![param("v", "", true, ParameterKind::String)]),
    );
    assert_eq!(
        refusal_of(&dialect(), &request),
        ClientError::StructuredOutputWithToolsNotSupported
    );
}

#[test]
fn given_a_structured_output_and_tool_calls_in_history_but_no_tool_when_encoded_then_it_is_accepted()
 {
    let request = request_with(
        None,
        vec![
            user("go"),
            assistant(vec![tool_call("t1", "lookup", json!({}))]),
            user_blocks(vec![tool_result("t1", "lookup", &["r"], false)]),
        ],
        vec![],
        structured(None, vec![]),
    );
    assert!(dialect().encode(&request).is_ok());
}

#[test]
fn given_a_plain_output_with_tools_when_encoded_then_it_is_accepted() {
    let request = request_with(
        None,
        vec![user("a")],
        vec![tool("lookup", "", vec![])],
        OutputMode::Plain,
    );
    assert!(dialect().encode(&request).is_ok());
}

#[test]
fn given_a_refused_request_when_encoded_then_no_encoded_request_is_given() {
    let request = request(vec![user("a"), assistant_text("b")]);
    assert!(dialect().encode(&request).is_err());
}
