use std::time::Duration;

use br_llm_clients_core::{CacheRequirement, ClientError, ReasoningLevel};
use br_llm_messages::{DeltaOp, ResponseEnding, ResponseMetadata};

use crate::call_log::{CallState, Served};
use crate::codes::{CODE_INVALID_REPLY, CODE_SCRIPT_EXHAUSTED};
use crate::response_support::*;
use crate::test_support::*;
use crate::{MockClient, Reply, Rule};

fn no_tools_mock() -> MockClient {
    MockClient::new(settings(), caps(|spec| spec.tools = false))
}

#[test]
fn given_a_refused_call_when_made_then_the_outcome_has_the_error_and_a_report_of_zero_duration_only()
 {
    let client = no_tools_mock();
    client.push_reply(Reply::text("never").build().unwrap());
    let (outcome, sink) = run(&client, &tools_request());
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
    let client = no_tools_mock();
    client.push_reply(Reply::text("kept").build().unwrap());
    client.push_rule(
        always_rule(Reply::failure(ClientError::Transport).build().unwrap())
            .with_limit(2)
            .unwrap(),
    );
    run(&client, &tools_request());
    assert_eq!(client.unused_entries(), 1);
    assert_eq!(client.unused_uses(), 2);
}

#[test]
fn given_a_refused_call_when_made_then_no_identity_is_drawn_from_the_source() {
    let source = crate::SequentialIdSource::new();
    let client = MockClient::builder(settings(), caps(|spec| spec.tools = false))
        .id_source(source.clone())
        .build();
    run(&client, &tools_request());
    assert_eq!(source.next_number(), 0);
}

#[test]
fn given_a_refused_call_when_logged_then_it_shows_the_refusal_and_the_finished_outcome() {
    let client = MockClient::new(settings(), caps(|spec| spec.caching = false)).with_settings(
        settings_with(ReasoningLevel::Off, CacheRequirement::Required, 10),
    );
    let (outcome, _) = run(&client, &request());
    let log = client.log();
    assert_eq!(
        log[0].served(),
        &Served::Refusal(ClientError::CacheNotSupported)
    );
    assert_eq!(log[0].state(), &CallState::Failed(outcome));
}

#[test]
fn given_a_rule_that_matches_a_pair_of_settings_when_the_pair_is_refused_then_the_log_shows_a_rule_and_not_a_refusal()
 {
    let client = mock();
    client.push_rule(Rule::new(
        |call| {
            call.settings().reasoning() == ReasoningLevel::Max
                && call.settings().max_output_tokens().get() > 100
        },
        Reply::failure(ClientError::MaxOutputTokensTooLarge { limit: Some(100) })
            .build()
            .unwrap(),
    ));
    client.push_reply(Reply::text("fine").build().unwrap());
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
    assert!(run(&client, &request()).0.result().is_ok());
}

#[test]
fn given_a_success_refused_against_the_request_when_called_then_all_deltas_but_completed_are_sent_and_the_report_is_kept()
 {
    let client = mock();
    let metadata = ResponseMetadata::new()
        .with_usage(usage(4, 6))
        .with_model(model("answering"));
    let response = response_of(
        vec![structured_content(r#"{"a":1}"#)],
        ResponseEnding::finished(),
        metadata,
    );
    client.push_reply(
        Reply::success(response.clone())
            .request_id("req-9")
            .time_to_first_event(Duration::from_millis(3))
            .total_duration(Duration::from_millis(7))
            .build()
            .unwrap(),
    );
    let request_value = request();
    let (outcome, sink) = run(&client, &request_value);
    assert!(matches!(
        error_of(&outcome),
        ClientError::MalformedResponse { .. }
    ));
    let mut all = expected_deltas(&request_value, &response, &crate::Chunking::none());
    assert!(matches!(
        all.pop().unwrap().op(),
        DeltaOp::ResponseCompleted { .. }
    ));
    assert_eq!(sink.deltas(), all);
    let report = outcome.report();
    assert_eq!(report.model(), Some(&model("answering")));
    assert_eq!(report.request_id(), Some("req-9"));
    assert_eq!(report.time_to_first_event(), Some(Duration::from_millis(3)));
    assert_eq!(report.total_duration(), Duration::from_millis(7));
    assert_eq!(report.usage(), Some(&usage(4, 6)));
}

#[test]
fn given_a_blocked_success_when_called_then_content_blocked_is_given_after_the_deltas() {
    let client = mock();
    let response = response_ending(vec![thinking_content()], ResponseEnding::refusal());
    client.push_reply(Reply::success(response).build().unwrap());
    let (outcome, sink) = run(&client, &request());
    assert_eq!(error_of(&outcome), ClientError::ContentBlocked);
    assert!(!sink.is_empty());
}

#[test]
fn given_the_codes_when_listed_then_they_start_with_mock_and_exhaustion_is_not_retryable() {
    for code in [CODE_INVALID_REPLY, CODE_SCRIPT_EXHAUSTED] {
        assert!(code.starts_with("mock."));
    }
    let (outcome, _) = run(&mock(), &request());
    assert!(!error_of(&outcome).is_retryable());
}

#[test]
fn given_a_complete_call_when_made_then_the_request_is_not_changed() {
    let client = mock();
    client.push_reply(Reply::text("ok").build().unwrap());
    let request_value = request();
    let before = request_value.clone();
    run(&client, &request_value);
    assert_eq!(request_value, before);
}
