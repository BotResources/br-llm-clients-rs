use br_llm_clients_core::{ClientError, ClientRequest, OutputMode};
use br_llm_messages::{AssistantBlock, RedactedThinking, Step, StopReason, Thinking};
use serde_json::json;

use crate::step_check::check;
use crate::test_support::*;

fn thinking_block() -> AssistantBlock {
    AssistantBlock::Thinking(Thinking {
        text: "hmm".to_owned(),
        signature: None,
    })
}

fn redacted_block() -> AssistantBlock {
    AssistantBlock::RedactedThinking(RedactedThinking::new("zz").unwrap())
}

fn text_block() -> AssistantBlock {
    AssistantBlock::Text {
        text: text("words"),
    }
}

fn structured_block() -> AssistantBlock {
    AssistantBlock::Structured {
        value: json!({ "a": 1 }),
    }
}

fn call_block(id: &str) -> AssistantBlock {
    tool_call(id, "search", json!({}))
}

fn plain() -> ClientRequest {
    request()
}

fn structured() -> ClientRequest {
    request_of(vec![user_text("go")], vec![], shape())
}

fn structured_with_tools() -> ClientRequest {
    request_of(vec![user_text("go")], vec![tool()], shape())
}

fn verdict(request_value: &ClientRequest, step_value: &Step) -> Result<(), ClientError> {
    check(&everything_caps(), request_value, step_value)
}

fn awaiting(content: Vec<AssistantBlock>) -> Step {
    step(content, StopReason::AwaitingToolResults)
}

fn ended(content: Vec<AssistantBlock>) -> Step {
    step(content, StopReason::EndTurn)
}

fn is_malformed(result: Result<(), ClientError>) -> bool {
    matches!(result, Err(ClientError::MalformedResponse { .. }))
}

#[test]
fn given_a_step_with_only_thinking_when_the_stop_is_a_filter_or_refusal_then_content_blocked() {
    for reason in [StopReason::ContentFilter, StopReason::Refusal] {
        let step_value = step(vec![thinking_block(), redacted_block()], reason);
        assert_eq!(
            verdict(&plain(), &step_value),
            Err(ClientError::ContentBlocked)
        );
    }
}

#[test]
fn given_a_step_with_only_thinking_when_the_stop_is_anything_else_then_reply_unusable_carries_it() {
    for reason in [
        StopReason::EndTurn,
        StopReason::MaxTokens,
        StopReason::StopSequence,
        StopReason::Other {
            reason: "odd".to_owned(),
        },
    ] {
        let step_value = step(vec![thinking_block()], reason.clone());
        assert_eq!(
            verdict(&plain(), &step_value),
            Err(ClientError::ReplyUnusable {
                stop_reason: reason
            })
        );
    }
}

#[test]
fn given_an_empty_step_in_structured_mode_when_checked_then_the_first_rule_still_decides() {
    let step_value = step(vec![thinking_block()], StopReason::MaxTokens);
    assert_eq!(
        verdict(&structured(), &step_value),
        Err(ClientError::ReplyUnusable {
            stop_reason: StopReason::MaxTokens
        })
    );
}

#[test]
fn given_structured_mode_without_value_or_call_when_blocked_then_content_blocked_whatever_the_text()
{
    for reason in [StopReason::ContentFilter, StopReason::Refusal] {
        let step_value = step(vec![text_block()], reason);
        assert_eq!(
            verdict(&structured(), &step_value),
            Err(ClientError::ContentBlocked)
        );
    }
}

#[test]
fn given_plain_mode_with_text_when_blocked_then_the_step_is_accepted_as_given() {
    let step_value = step(vec![text_block()], StopReason::ContentFilter);
    assert_eq!(verdict(&plain(), &step_value), Ok(()));
}

#[test]
fn given_structured_mode_when_the_step_holds_one_value_only_then_accepted() {
    let step_value = ended(vec![thinking_block(), structured_block(), redacted_block()]);
    assert_eq!(verdict(&structured(), &step_value), Ok(()));
}

#[test]
fn given_structured_mode_when_the_step_is_not_exactly_one_value_then_malformed() {
    let cases = [
        ended(vec![text_block()]),
        ended(vec![structured_block(), text_block()]),
        ended(vec![structured_block(), structured_block()]),
        awaiting(vec![call_block("a")]),
        awaiting(vec![structured_block(), call_block("a")]),
    ];
    for step_value in cases {
        assert!(is_malformed(verdict(&structured(), &step_value)));
    }
}

#[test]
fn given_structured_mode_with_tools_when_the_step_holds_calls_and_text_then_accepted() {
    let step_value = awaiting(vec![
        text_block(),
        thinking_block(),
        call_block("a"),
        call_block("b"),
    ]);
    assert_eq!(verdict(&structured_with_tools(), &step_value), Ok(()));
}

#[test]
fn given_structured_mode_with_tools_when_the_step_holds_a_value_then_accepted() {
    let step_value = ended(vec![structured_block()]);
    assert_eq!(verdict(&structured_with_tools(), &step_value), Ok(()));
}

#[test]
fn given_structured_mode_with_tools_when_the_step_mixes_value_and_calls_then_malformed() {
    let step_value = awaiting(vec![structured_block(), call_block("a")]);
    assert!(is_malformed(verdict(&structured_with_tools(), &step_value)));
}

#[test]
fn given_structured_mode_with_tools_when_the_step_holds_only_text_then_malformed() {
    let step_value = ended(vec![text_block()]);
    assert!(is_malformed(verdict(&structured_with_tools(), &step_value)));
}

#[test]
fn given_structured_mode_without_declared_tools_when_the_step_calls_then_malformed() {
    assert!(is_malformed(verdict(
        &structured(),
        &awaiting(vec![call_block("a")])
    )));
}

#[test]
fn given_plain_mode_when_the_step_holds_a_structured_block_then_malformed() {
    let step_value = ended(vec![text_block(), structured_block()]);
    assert!(is_malformed(verdict(&plain(), &step_value)));
}

#[test]
fn given_a_call_id_equal_to_one_of_the_request_when_checked_then_malformed_names_id_and_the_mock() {
    let history = request_of(
        vec![user_text("go"), assistant_call("dup"), user_result("dup")],
        vec![tool()],
        OutputMode::Plain,
    );
    let result = verdict(&history, &calls_step(&["fresh", "dup"]));
    let Err(ClientError::MalformedResponse { detail }) = result else {
        panic!("expected a malformed response");
    };
    assert!(detail.contains("dup"));
    assert!(detail.contains("mock"));
}

#[test]
fn given_a_fresh_call_id_when_checked_then_accepted() {
    let history = request_of(
        vec![user_text("go"), assistant_call("old"), user_result("old")],
        vec![tool()],
        OutputMode::Plain,
    );
    assert_eq!(verdict(&history, &calls_step(&["new"])), Ok(()));
}

#[test]
fn given_a_repeated_id_in_structured_mode_when_checked_then_the_id_rule_still_applies() {
    let history = request_of(
        vec![user_text("go"), assistant_call("dup"), user_result("dup")],
        vec![tool()],
        shape(),
    );
    assert!(is_malformed(verdict(&history, &calls_step(&["dup"]))));
}

#[test]
fn given_a_structured_error_and_a_repeated_id_when_checked_then_the_earlier_rule_decides() {
    let history = request_of(
        vec![user_text("go"), assistant_call("dup"), user_result("dup")],
        vec![tool()],
        OutputMode::Plain,
    );
    let step_value = step(
        vec![structured_block(), call_block("dup")],
        StopReason::AwaitingToolResults,
    );
    let Err(ClientError::MalformedResponse { detail }) = verdict(&history, &step_value) else {
        panic!("expected a malformed response");
    };
    assert!(!detail.contains("dup"));
}

#[test]
fn given_a_plain_text_step_when_checked_then_accepted() {
    assert_eq!(verdict(&plain(), &text_step("ok")), Ok(()));
}
