use br_llm_messages::{
    AssistantBlock, ToolCall, ToolResult, ToolResultBlock, WireMessage, WireUserBlock,
};

use crate::ClientRequest;
use crate::client_error::ClientError;
use crate::output_mode::OutputMode;
use crate::test_support::{call_id, text};

fn user_text(value: &str) -> WireMessage {
    WireMessage::User {
        content: vec![WireUserBlock::Text { text: text(value) }],
    }
}

fn assistant_calling(ids: &[&str]) -> WireMessage {
    WireMessage::Assistant {
        content: ids
            .iter()
            .map(|id| {
                AssistantBlock::ToolCall(ToolCall {
                    id: call_id(id),
                    name: br_llm_messages::ToolName::new("search").unwrap(),
                    arguments: serde_json::json!({}),
                })
            })
            .collect(),
    }
}

fn assistant_text(value: &str) -> WireMessage {
    WireMessage::Assistant {
        content: vec![AssistantBlock::Text { text: text(value) }],
    }
}

fn user_results(ids: &[&str]) -> WireMessage {
    WireMessage::User {
        content: ids
            .iter()
            .map(|id| {
                WireUserBlock::ToolResult(ToolResult::new(
                    call_id(id),
                    br_llm_messages::ToolName::new("search").unwrap(),
                    vec![ToolResultBlock::text(text("done"))],
                    false,
                ))
            })
            .collect(),
    }
}

fn relay(value: &str) -> WireMessage {
    WireMessage::Relay {
        content: vec![text(value)],
    }
}

fn verdict(messages: Vec<WireMessage>) -> Result<(), ClientError> {
    ClientRequest::new(None, messages, vec![], OutputMode::Plain)
        .unwrap()
        .check_messages()
}

fn assert_invalid(messages: Vec<WireMessage>) {
    assert!(
        matches!(verdict(messages), Err(ClientError::InvalidMessages { .. })),
        "expected InvalidMessages"
    );
}

#[test]
fn given_a_list_starting_with_an_assistant_message_when_checked_then_invalid() {
    assert_invalid(vec![assistant_text("a"), user_text("b")]);
}

#[test]
fn given_a_plain_exchange_when_checked_then_valid() {
    assert_eq!(
        verdict(vec![user_text("a"), assistant_text("b"), user_text("c")]),
        Ok(())
    );
}

#[test]
fn given_a_tool_call_answered_once_in_the_next_user_message_when_checked_then_valid() {
    assert_eq!(
        verdict(vec![
            user_text("a"),
            assistant_calling(&["c1", "c2"]),
            user_results(&["c2", "c1"]),
        ]),
        Ok(())
    );
}

#[test]
fn given_a_trailing_relay_after_the_results_when_checked_then_valid() {
    assert_eq!(
        verdict(vec![
            user_text("a"),
            assistant_calling(&["c1"]),
            user_results(&["c1"]),
            relay("news"),
        ]),
        Ok(())
    );
}

#[test]
fn given_a_tool_call_without_any_result_when_checked_then_invalid() {
    assert_invalid(vec![
        user_text("a"),
        assistant_calling(&["c1"]),
        user_text("b"),
    ]);
}

#[test]
fn given_a_tool_call_at_the_end_of_the_list_when_checked_then_invalid() {
    assert_invalid(vec![user_text("a"), assistant_calling(&["c1"])]);
}

#[test]
fn given_two_calls_and_one_result_when_checked_then_invalid() {
    assert_invalid(vec![
        user_text("a"),
        assistant_calling(&["c1", "c2"]),
        user_results(&["c1"]),
    ]);
}

#[test]
fn given_a_result_given_twice_when_checked_then_invalid() {
    assert_invalid(vec![
        user_text("a"),
        assistant_calling(&["c1"]),
        user_results(&["c1", "c1"]),
    ]);
}

#[test]
fn given_a_result_without_an_earlier_tool_call_when_checked_then_invalid() {
    assert_invalid(vec![
        user_text("a"),
        assistant_text("b"),
        user_results(&["ghost"]),
    ]);
}

#[test]
fn given_a_result_for_a_call_declared_later_when_checked_then_invalid() {
    assert_invalid(vec![
        user_results(&["c1"]),
        assistant_calling(&["c1"]),
        user_results(&["c1"]),
    ]);
}

#[test]
fn given_a_result_for_an_unknown_id_beside_a_valid_one_when_checked_then_invalid() {
    assert_invalid(vec![
        user_text("a"),
        assistant_calling(&["c1"]),
        user_results(&["c1", "ghost"]),
    ]);
}

#[test]
fn given_a_call_answered_again_in_a_later_user_message_when_checked_then_invalid() {
    assert_invalid(vec![
        user_text("a"),
        assistant_calling(&["c1"]),
        user_results(&["c1"]),
        assistant_text("b"),
        user_results(&["c1"]),
    ]);
}

#[test]
fn given_the_same_id_on_two_calls_of_one_message_answered_by_one_result_when_checked_then_invalid()
{
    let error = verdict(vec![
        user_text("a"),
        assistant_calling(&["c1", "c1"]),
        user_results(&["c1"]),
    ])
    .unwrap_err();
    assert!(
        matches!(error, ClientError::InvalidMessages { reason } if reason.contains("c1") && reason.contains("more than once"))
    );
}

#[test]
fn given_an_id_reused_by_a_call_of_a_later_turn_when_checked_then_invalid() {
    let error = verdict(vec![
        user_text("a"),
        assistant_calling(&["c1"]),
        user_results(&["c1"]),
        assistant_calling(&["c1"]),
        user_results(&["c1"]),
    ])
    .unwrap_err();
    assert!(
        matches!(error, ClientError::InvalidMessages { reason } if reason.contains("c1") && reason.contains("more than once"))
    );
}

#[test]
fn given_distinct_ids_across_turns_when_checked_then_valid() {
    assert_eq!(
        verdict(vec![
            user_text("a"),
            assistant_calling(&["c1"]),
            user_results(&["c1"]),
            assistant_calling(&["c2"]),
            user_results(&["c2"]),
        ]),
        Ok(())
    );
}

#[test]
fn given_a_list_ending_with_an_assistant_text_when_checked_then_left_to_the_client_library() {
    assert_eq!(verdict(vec![user_text("a"), assistant_text("b")]), Ok(()));
}

#[test]
fn given_a_call_to_an_undeclared_tool_when_checked_then_left_to_the_client_library() {
    assert_eq!(
        verdict(vec![
            user_text("a"),
            assistant_calling(&["c1"]),
            user_results(&["c1"]),
        ]),
        Ok(())
    );
}

#[test]
fn given_an_invalid_list_when_refused_then_the_reason_is_text_and_a_retry_cannot_help() {
    let error = verdict(vec![assistant_text("a"), user_text("b")]).unwrap_err();
    assert!(!error.is_retryable());
    assert!(matches!(error, ClientError::InvalidMessages { reason } if !reason.is_empty()));
}
