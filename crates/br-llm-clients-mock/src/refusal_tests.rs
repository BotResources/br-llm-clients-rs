use std::time::Duration;

use br_llm_clients_core::{CacheRequirement, ClientError, OutputMode, ReasoningLevel};
use br_llm_messages::{AssistantBlock, StopReason, StreamEvent};
use serde_json::json;

use crate::call_log::{CallState, Served};
use crate::chunking::Chunking;
use crate::codes::{CODE_INVALID_REPLY, CODE_SCRIPT_EXHAUSTED};
use crate::derive_events::derive_events;
use crate::mock_client::MockClient;
use crate::reply::Reply;
use crate::rule::Rule;
use crate::test_support::*;
use crate::{Accepted, CapabilitiesSpec};

fn strict_mock(change: impl FnOnce(&mut CapabilitiesSpec)) -> MockClient {
    MockClient::new(settings(), caps(change))
}

#[test]
fn given_a_refused_call_when_made_then_the_outcome_has_the_error_and_a_report_of_zero_duration_only()
 {
    let client = strict_mock(|spec| spec.tools = false);
    client.push_reply(Reply::success(text_step("never")).build().unwrap());
    let with_tool = request_of(vec![user_text("hi")], vec![tool()], OutputMode::Plain);
    let (outcome, sink) = run(&client, &with_tool);
    assert_eq!(error_of(&outcome), ClientError::ToolsNotSupported);
    let report = outcome.report();
    assert_eq!(report.total_duration(), Duration::ZERO);
    assert_eq!(report.usage(), None);
    assert_eq!(report.model(), None);
    assert_eq!(report.request_id(), None);
    assert_eq!(report.time_to_first_event(), None);
    assert!(sink.is_empty());
}

#[test]
fn given_a_refused_call_when_made_then_no_entry_and_no_use_is_taken() {
    let client = strict_mock(|spec| spec.tools = false);
    client.push_reply(Reply::success(text_step("kept")).build().unwrap());
    client.push_rule(
        always_rule(Reply::failure(ClientError::Transport).build().unwrap())
            .with_limit(2)
            .unwrap(),
    );
    let with_tool = request_of(vec![user_text("hi")], vec![tool()], OutputMode::Plain);
    run(&client, &with_tool);
    assert_eq!(client.unused_entries(), 1);
    assert_eq!(client.unused_uses(), 2);
}

#[test]
fn given_a_refused_call_when_logged_then_it_shows_the_refusal_and_the_finished_outcome() {
    let client = strict_mock(|spec| spec.caching = false);
    let cached = settings_with(ReasoningLevel::Off, CacheRequirement::Required, 10);
    let client = client.with_settings(cached);
    let (outcome, _) = run(&client, &request());
    let log = client.log();
    assert_eq!(
        log[0].served(),
        &Served::Refusal(ClientError::CacheNotSupported)
    );
    assert_eq!(log[0].state(), &CallState::Failed(outcome));
}

#[test]
fn given_a_rule_that_matches_the_settings_when_a_pair_is_refused_then_the_log_shows_a_rule_and_not_a_refusal()
 {
    let client = mock();
    let refusing = Rule::new(
        |call| {
            call.settings().reasoning() == ReasoningLevel::Max
                && call.settings().max_output_tokens().get() > 100
        },
        Reply::failure(ClientError::MaxOutputTokensTooLarge { limit: Some(100) })
            .build()
            .unwrap(),
    );
    client.push_rule(refusing);
    client.push_reply(Reply::success(text_step("fine")).build().unwrap());
    let pair = client.with_settings(settings_with(
        ReasoningLevel::Max,
        CacheRequirement::NotRequired,
        500,
    ));
    let (outcome, _) = run(&pair, &request());
    assert_eq!(
        error_of(&outcome),
        ClientError::MaxOutputTokensTooLarge { limit: Some(100) }
    );
    assert_eq!(client.log()[0].served(), &Served::Rule { position: 0 });
    let (other, _) = run(&client, &request());
    assert!(other.result().is_ok());
}

#[test]
fn given_an_empty_set_of_levels_when_a_call_with_valid_messages_is_made_then_reasoning_not_supported()
 {
    let client = strict_mock(|spec| spec.reasoning_levels = Accepted::Only(vec![]));
    let (outcome, _) = run(&client, &request());
    assert!(matches!(
        error_of(&outcome),
        ClientError::ReasoningNotSupported { .. }
    ));
}

#[test]
fn given_a_success_refused_against_the_request_when_called_then_all_events_but_finish_are_sent() {
    let client = mock();
    let step_value = step(
        vec![AssistantBlock::Structured {
            value: json!({ "a": 1 }),
        }],
        StopReason::EndTurn,
    );
    let all = derive_events(&step_value, &Chunking::none())
        .unwrap()
        .into_events();
    client.push_reply(
        Reply::success(step_with_usage_of(step_value, usage(4, 6)))
            .model(model("answering"))
            .request_id("req-9")
            .time_to_first_event(Duration::from_millis(3))
            .total_duration(Duration::from_millis(7))
            .build()
            .unwrap(),
    );
    let (outcome, sink) = run(&client, &request());
    assert!(matches!(
        error_of(&outcome),
        ClientError::MalformedResponse { .. }
    ));
    assert_eq!(sink.events(), all[..all.len() - 1]);
    let report = outcome.report();
    assert_eq!(report.model(), Some(&model("answering")));
    assert_eq!(report.request_id(), Some("req-9"));
    assert_eq!(report.time_to_first_event(), Some(Duration::from_millis(3)));
    assert_eq!(report.total_duration(), Duration::from_millis(7));
    assert_eq!(report.usage(), Some(&usage(4, 6)));
}

fn step_with_usage_of(
    step_value: br_llm_messages::Step,
    usage_value: br_llm_messages::Usage,
) -> br_llm_messages::Step {
    br_llm_messages::Step::new(
        step_value.content().to_vec(),
        step_value.stop_reason().clone(),
        Some(usage_value),
        None,
    )
    .unwrap()
}

#[test]
fn given_an_unusable_success_when_called_then_the_error_of_core_is_given_with_the_reply_report() {
    let client = mock();
    let only_thinking = step(
        vec![AssistantBlock::Thinking(br_llm_messages::Thinking {
            text: "hm".to_owned(),
            signature: None,
        })],
        StopReason::MaxTokens,
    );
    client.push_reply(Reply::success(only_thinking).build().unwrap());
    let (outcome, sink) = run(&client, &request());
    assert_eq!(
        error_of(&outcome),
        ClientError::ReplyUnusable {
            stop_reason: StopReason::MaxTokens
        }
    );
    assert!(
        !sink
            .events()
            .iter()
            .any(|event| matches!(event, StreamEvent::Finish { .. }))
    );
}

#[test]
fn given_the_codes_when_listed_then_they_start_with_mock_and_exhaustion_is_not_retryable() {
    for code in [CODE_INVALID_REPLY, CODE_SCRIPT_EXHAUSTED] {
        assert!(code.starts_with("mock."));
    }
    let client = mock();
    let (outcome, _) = run(&client, &request());
    assert!(!error_of(&outcome).is_retryable());
}

#[test]
fn given_the_everything_capabilities_when_a_complete_call_is_made_then_the_request_is_not_changed()
{
    let client = MockClient::new(settings(), everything_caps());
    client.push_reply(Reply::success(text_step("ok")).build().unwrap());
    let request_value = request();
    let before = request_value.clone();
    run(&client, &request_value);
    assert_eq!(request_value, before);
}
