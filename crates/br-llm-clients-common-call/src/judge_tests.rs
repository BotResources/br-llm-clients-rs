use br_llm_clients_core::{ClientError, OutputMode};
use br_llm_messages::StopReason::{
    self, AwaitingToolResults, ContentFilter, EndTurn, MaxTokens, Refusal, StopSequence,
};

use crate::judge::{Held, Json, judge};
use crate::test_requests::structured_request;

fn text() -> Held {
    Held::Text { nonblank: true }
}

fn blank() -> Held {
    Held::Text { nonblank: false }
}

fn call() -> Held {
    Held::ToolCall(Json::Complete)
}

fn value() -> Held {
    Held::Structured(Json::Complete)
}

fn shape() -> OutputMode {
    structured_request(false).output().clone()
}

fn plain(held: &[Held], stop: StopReason) -> Result<StopReason, ClientError> {
    judge(held, &stop, &OutputMode::Plain, false)
}

fn structured(held: &[Held], stop: StopReason, tools: bool) -> Result<StopReason, ClientError> {
    judge(held, &stop, &shape(), tools)
}

fn other() -> StopReason {
    StopReason::Other {
        reason: "x".to_owned(),
    }
}

fn is_malformed(result: &Result<StopReason, ClientError>) -> bool {
    matches!(result, Err(ClientError::MalformedResponse { .. }))
}

#[test]
fn given_json_that_is_incomplete_when_judged_then_the_stop_reason_decides() {
    for incomplete in [
        Held::ToolCall(Json::Incomplete),
        Held::Structured(Json::Incomplete),
    ] {
        let held = [text(), incomplete];
        assert_eq!(
            plain(&held, ContentFilter),
            Err(ClientError::ContentBlocked)
        );
        assert_eq!(plain(&held, Refusal), Err(ClientError::ContentBlocked));
        assert_eq!(
            plain(&held, MaxTokens),
            Err(ClientError::ReplyUnusable {
                stop_reason: MaxTokens
            })
        );
        for stop in [EndTurn, AwaitingToolResults, StopSequence, other()] {
            assert!(is_malformed(&plain(&held, stop.clone())), "{stop}");
        }
    }
}

#[test]
fn given_json_that_is_not_valid_when_judged_then_the_reply_is_malformed_whatever_the_stop_reason() {
    let held = [Held::ToolCall(Json::Malformed("bad".to_owned()))];
    for stop in [
        EndTurn,
        MaxTokens,
        ContentFilter,
        Refusal,
        AwaitingToolResults,
    ] {
        assert!(is_malformed(&plain(&held, stop.clone())), "{stop}");
    }
}

#[test]
fn given_incomplete_and_invalid_json_together_when_judged_then_incomplete_is_judged_first() {
    let held = [
        Held::ToolCall(Json::Malformed("bad".to_owned())),
        Held::Structured(Json::Incomplete),
    ];
    assert_eq!(
        plain(&held, MaxTokens),
        Err(ClientError::ReplyUnusable {
            stop_reason: MaxTokens
        })
    );
}

#[test]
fn given_a_reply_with_nothing_usable_when_judged_then_it_is_unusable_with_the_stop_reason_as_given()
{
    for held in [
        vec![],
        vec![blank()],
        vec![Held::Thinking],
        vec![Held::Redacted, blank()],
    ] {
        for stop in [
            EndTurn,
            MaxTokens,
            StopSequence,
            other(),
            AwaitingToolResults,
        ] {
            assert_eq!(
                plain(&held, stop.clone()),
                Err(ClientError::ReplyUnusable { stop_reason: stop }),
            );
        }
    }
}

#[test]
fn given_a_filtered_reply_with_nothing_usable_when_judged_then_the_content_is_blocked() {
    for held in [vec![], vec![blank()], vec![Held::Thinking]] {
        assert_eq!(
            plain(&held, ContentFilter),
            Err(ClientError::ContentBlocked)
        );
        assert_eq!(plain(&held, Refusal), Err(ClientError::ContentBlocked));
    }
}

#[test]
fn given_a_filtered_reply_that_holds_something_when_judged_then_it_is_a_normal_step() {
    assert_eq!(plain(&[text()], ContentFilter), Ok(ContentFilter));
    assert_eq!(plain(&[text()], Refusal), Ok(Refusal));
    assert_eq!(plain(&[value()], ContentFilter), Ok(ContentFilter));
    assert_eq!(plain(&[call()], ContentFilter), Ok(AwaitingToolResults));
}

#[test]
fn given_a_content_and_a_stop_reason_when_judged_then_the_stop_reason_is_set_from_the_content() {
    assert_eq!(plain(&[text(), call()], EndTurn), Ok(AwaitingToolResults));
    assert_eq!(plain(&[call()], MaxTokens), Ok(AwaitingToolResults));
    assert_eq!(
        plain(&[call()], AwaitingToolResults),
        Ok(AwaitingToolResults)
    );
    assert_eq!(plain(&[text()], AwaitingToolResults), Ok(EndTurn));
    assert_eq!(plain(&[text()], MaxTokens), Ok(MaxTokens));
    assert_eq!(plain(&[text()], StopSequence), Ok(StopSequence));
    assert_eq!(plain(&[text()], other()), Ok(other()));
    assert_eq!(plain(&[text()], EndTurn), Ok(EndTurn));
}

#[test]
fn given_the_structured_mode_when_the_reply_holds_one_value_then_it_is_valid_with_any_stop_but_the_tool_one()
 {
    assert_eq!(structured(&[value()], EndTurn, false), Ok(EndTurn));
    assert_eq!(structured(&[value()], MaxTokens, false), Ok(MaxTokens));
    assert_eq!(
        structured(&[value()], AwaitingToolResults, false),
        Ok(EndTurn)
    );
    assert_eq!(
        structured(&[Held::Thinking, value(), blank()], EndTurn, false),
        Ok(EndTurn)
    );
}

#[test]
fn given_the_structured_mode_when_the_reply_is_not_one_value_then_it_is_malformed() {
    assert!(is_malformed(&structured(
        &[value(), text()],
        EndTurn,
        false
    )));
    assert!(is_malformed(&structured(&[text()], EndTurn, false)));
    assert!(is_malformed(&structured(
        &[value(), value()],
        EndTurn,
        false
    )));
    assert!(is_malformed(&structured(&[value(), call()], EndTurn, true)));
}

#[test]
fn given_the_structured_mode_when_tools_are_declared_then_tool_calls_can_come_instead_with_text() {
    assert_eq!(
        structured(&[call()], EndTurn, true),
        Ok(AwaitingToolResults)
    );
    assert_eq!(
        structured(&[text(), call()], EndTurn, true),
        Ok(AwaitingToolResults)
    );
    assert!(is_malformed(&structured(&[call()], EndTurn, false)));
}

#[test]
fn given_the_structured_mode_when_nothing_is_usable_then_unusable_or_blocked_wins_over_malformed() {
    assert_eq!(
        structured(&[Held::Thinking, blank()], EndTurn, false),
        Err(ClientError::ReplyUnusable {
            stop_reason: EndTurn
        })
    );
    assert_eq!(
        structured(&[], ContentFilter, false),
        Err(ClientError::ContentBlocked)
    );
}

#[test]
fn given_the_structured_mode_when_a_filtered_reply_has_no_value_and_no_call_then_it_is_blocked_whatever_text_it_holds()
 {
    assert_eq!(
        structured(&[text()], ContentFilter, false),
        Err(ClientError::ContentBlocked)
    );
    assert_eq!(
        structured(&[text()], Refusal, true),
        Err(ClientError::ContentBlocked)
    );
    assert_eq!(
        structured(&[value()], ContentFilter, false),
        Ok(ContentFilter)
    );
    assert_eq!(
        structured(&[call()], ContentFilter, true),
        Ok(AwaitingToolResults)
    );
}

#[test]
fn given_the_plain_mode_when_the_reply_holds_a_structured_value_then_no_rule_refuses_it() {
    assert_eq!(plain(&[value(), text()], EndTurn), Ok(EndTurn));
}
