use br_llm_clients_core::{ClientError, OutputMode};
use br_llm_messages::ResponseEnding;

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

fn plain(held: &[Held], stop: ResponseEnding) -> Result<ResponseEnding, ClientError> {
    judge(held, &stop, &OutputMode::Plain, false)
}

fn structured(
    held: &[Held],
    stop: ResponseEnding,
    tools: bool,
) -> Result<ResponseEnding, ClientError> {
    judge(held, &stop, &shape(), tools)
}

fn other() -> ResponseEnding {
    ResponseEnding::unclassified("x").expect("an ending")
}

fn finished() -> ResponseEnding {
    ResponseEnding::finished()
}

fn tool_use() -> ResponseEnding {
    ResponseEnding::tool_use()
}

fn exhausted() -> ResponseEnding {
    ResponseEnding::output_exhaustion()
}

fn filtering() -> ResponseEnding {
    ResponseEnding::filtering()
}

fn refusal() -> ResponseEnding {
    ResponseEnding::refusal()
}

fn stop_seq() -> ResponseEnding {
    ResponseEnding::stop_sequence()
}

fn is_malformed(result: &Result<ResponseEnding, ClientError>) -> bool {
    matches!(result, Err(ClientError::MalformedResponse { .. }))
}

#[test]
fn given_json_that_is_incomplete_when_judged_then_the_stop_reason_decides() {
    for incomplete in [
        Held::ToolCall(Json::Incomplete),
        Held::Structured(Json::Incomplete),
    ] {
        let held = [text(), incomplete];
        assert_eq!(plain(&held, filtering()), Err(ClientError::ContentBlocked));
        assert_eq!(plain(&held, refusal()), Err(ClientError::ContentBlocked));
        assert_eq!(
            plain(&held, exhausted()),
            Err(ClientError::ReplyUnusable {
                ending: exhausted()
            })
        );
        for stop in [finished(), tool_use(), stop_seq(), other()] {
            assert!(is_malformed(&plain(&held, stop.clone())), "{stop:?}");
        }
    }
}

#[test]
fn given_json_that_is_not_valid_when_judged_then_the_reply_is_malformed_whatever_the_stop_reason() {
    let held = [Held::ToolCall(Json::Malformed("bad".to_owned()))];
    for stop in [finished(), exhausted(), filtering(), refusal(), tool_use()] {
        assert!(is_malformed(&plain(&held, stop.clone())), "{stop:?}");
    }
}

#[test]
fn given_incomplete_and_invalid_json_together_when_judged_then_incomplete_is_judged_first() {
    let held = [
        Held::ToolCall(Json::Malformed("bad".to_owned())),
        Held::Structured(Json::Incomplete),
    ];
    assert_eq!(
        plain(&held, exhausted()),
        Err(ClientError::ReplyUnusable {
            ending: exhausted()
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
        for stop in [finished(), exhausted(), stop_seq(), other(), tool_use()] {
            assert_eq!(
                plain(&held, stop.clone()),
                Err(ClientError::ReplyUnusable { ending: stop }),
            );
        }
    }
}

#[test]
fn given_a_filtered_reply_with_nothing_usable_when_judged_then_the_content_is_blocked() {
    for held in [vec![], vec![blank()], vec![Held::Thinking]] {
        assert_eq!(plain(&held, filtering()), Err(ClientError::ContentBlocked));
        assert_eq!(plain(&held, refusal()), Err(ClientError::ContentBlocked));
    }
}

#[test]
fn given_a_filtered_reply_that_holds_something_when_judged_then_it_is_a_normal_step() {
    assert_eq!(plain(&[text()], filtering()), Ok(filtering()));
    assert_eq!(plain(&[text()], refusal()), Ok(refusal()));
    assert_eq!(plain(&[value()], filtering()), Ok(filtering()));
    assert_eq!(plain(&[call()], filtering()), Ok(tool_use()));
}

#[test]
fn given_a_content_and_a_stop_reason_when_judged_then_the_stop_reason_is_set_from_the_content() {
    assert_eq!(plain(&[text(), call()], finished()), Ok(tool_use()));
    assert_eq!(plain(&[call()], exhausted()), Ok(tool_use()));
    assert_eq!(plain(&[call()], tool_use()), Ok(tool_use()));
    assert_eq!(plain(&[text()], tool_use()), Ok(finished()));
    assert_eq!(plain(&[text()], exhausted()), Ok(exhausted()));
    assert_eq!(plain(&[text()], stop_seq()), Ok(stop_seq()));
    assert_eq!(plain(&[text()], other()), Ok(other()));
    assert_eq!(plain(&[text()], finished()), Ok(finished()));
}

#[test]
fn given_the_structured_mode_when_the_reply_holds_one_value_then_it_is_valid_with_any_stop_but_the_tool_one()
 {
    assert_eq!(structured(&[value()], finished(), false), Ok(finished()));
    assert_eq!(structured(&[value()], exhausted(), false), Ok(exhausted()));
    assert_eq!(structured(&[value()], tool_use(), false), Ok(finished()));
    assert_eq!(
        structured(&[Held::Thinking, value(), blank()], finished(), false),
        Ok(finished())
    );
}

#[test]
fn given_the_structured_mode_when_the_reply_is_not_one_value_then_it_is_malformed() {
    assert!(is_malformed(&structured(
        &[value(), text()],
        finished(),
        false
    )));
    assert!(is_malformed(&structured(&[text()], finished(), false)));
    assert!(is_malformed(&structured(
        &[value(), value()],
        finished(),
        false
    )));
    assert!(is_malformed(&structured(
        &[value(), call()],
        finished(),
        true
    )));
}

#[test]
fn given_the_structured_mode_when_tools_are_declared_then_tool_calls_can_come_instead_with_text() {
    assert_eq!(structured(&[call()], finished(), true), Ok(tool_use()));
    assert_eq!(
        structured(&[text(), call()], finished(), true),
        Ok(tool_use())
    );
    assert!(is_malformed(&structured(&[call()], finished(), false)));
}

#[test]
fn given_the_structured_mode_when_nothing_is_usable_then_unusable_or_blocked_wins_over_malformed() {
    assert_eq!(
        structured(&[Held::Thinking, blank()], finished(), false),
        Err(ClientError::ReplyUnusable { ending: finished() })
    );
    assert_eq!(
        structured(&[], filtering(), false),
        Err(ClientError::ContentBlocked)
    );
}

#[test]
fn given_the_structured_mode_when_a_filtered_reply_has_no_value_and_no_call_then_it_is_blocked_whatever_text_it_holds()
 {
    assert_eq!(
        structured(&[text()], filtering(), false),
        Err(ClientError::ContentBlocked)
    );
    assert_eq!(
        structured(&[text()], refusal(), true),
        Err(ClientError::ContentBlocked)
    );
    assert_eq!(structured(&[value()], filtering(), false), Ok(filtering()));
    assert_eq!(structured(&[call()], filtering(), true), Ok(tool_use()));
}

#[test]
fn given_the_plain_mode_when_the_reply_holds_a_structured_value_then_no_rule_refuses_it() {
    assert_eq!(plain(&[value(), text()], finished()), Ok(finished()));
}

#[test]
fn given_the_ending_provider_continuation_when_judged_then_the_reply_is_malformed() {
    let cont = ResponseEnding::provider_continuation();
    assert!(is_malformed(&plain(&[text()], cont.clone())));
    assert!(is_malformed(&plain(&[call()], cont)));
}
