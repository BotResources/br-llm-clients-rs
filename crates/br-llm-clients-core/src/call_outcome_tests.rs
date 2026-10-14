use std::time::Duration;

use br_llm_messages::{AssistantBlock, ModelId, Step, StopReason, ToolCall, ToolName};

use crate::build_error::BuildError;
use crate::test_support::{call_id, text_step, usage};
use crate::{CallOutcome, CallReport, ClientError};

fn report() -> CallReport {
    CallReport::new(Duration::from_millis(900))
}

#[test]
fn given_a_report_with_the_usage_of_the_step_when_a_success_is_built_then_accepted() {
    let step = text_step("hi", Some(usage(10, 5)));
    let outcome = CallOutcome::success(report().with_usage(usage(10, 5)), step.clone()).unwrap();
    assert_eq!(outcome.result(), Ok(&step));
}

#[test]
fn given_a_report_and_a_step_both_without_usage_when_a_success_is_built_then_accepted() {
    assert!(CallOutcome::success(report(), text_step("hi", None)).is_ok());
}

#[test]
fn given_a_report_usage_differing_from_the_step_when_a_success_is_built_then_refused() {
    let step = text_step("hi", Some(usage(10, 5)));
    assert_eq!(
        CallOutcome::success(report().with_usage(usage(10, 6)), step.clone()),
        Err(BuildError::UsageMismatch)
    );
    assert_eq!(
        CallOutcome::success(report(), step),
        Err(BuildError::UsageMismatch)
    );
}

#[test]
fn given_a_failure_when_built_then_the_report_is_present_with_its_own_usage() {
    let outcome = CallOutcome::failure(
        report().with_usage(usage(3, 1)),
        ClientError::StreamTruncated,
    );
    assert_eq!(outcome.report().usage(), Some(&usage(3, 1)));
    assert_eq!(outcome.result(), Err(&ClientError::StreamTruncated));
}

#[test]
fn given_a_failure_with_the_last_usage_reported_before_it_when_stored_and_loaded_then_accepted_whatever_the_usage()
 {
    let outcome = CallOutcome::failure(
        report().with_usage(usage(7, 2)),
        ClientError::StreamTruncated,
    );
    let json = serde_json::to_value(&outcome).unwrap();
    let loaded = serde_json::from_value::<CallOutcome>(json).unwrap();
    assert_eq!(loaded, outcome);
    assert_eq!(loaded.report().usage(), Some(&usage(7, 2)));
}

#[test]
fn given_a_full_report_when_read_then_each_item_comes_back() {
    let report = report()
        .with_usage(usage(1, 2))
        .with_model(ModelId::new("m").unwrap())
        .with_request_id("req-1")
        .with_time_to_first_event(Duration::from_millis(120));
    assert_eq!(report.total_duration(), Duration::from_millis(900));
    assert_eq!(
        report.time_to_first_event(),
        Some(Duration::from_millis(120))
    );
    assert_eq!(report.request_id(), Some("req-1"));
    assert_eq!(report.model().map(ModelId::as_str), Some("m"));
}

#[test]
fn given_a_report_with_only_the_total_duration_when_read_then_every_other_item_is_absent() {
    let report = report();
    assert!(report.usage().is_none());
    assert!(report.model().is_none());
    assert!(report.request_id().is_none());
    assert!(report.time_to_first_event().is_none());
}

#[test]
fn given_a_success_when_round_tripped_then_identical() {
    let step = text_step("hi", Some(usage(10, 5)));
    let full = report()
        .with_usage(usage(10, 5))
        .with_model(ModelId::new("m").unwrap())
        .with_request_id("req-1")
        .with_time_to_first_event(Duration::from_millis(120));
    let outcome = CallOutcome::success(full, step).unwrap();
    let json = serde_json::to_value(&outcome).unwrap();
    assert_eq!(json["schema"], serde_json::json!("br-llm-clients-core/1"));
    assert_eq!(
        serde_json::from_value::<CallOutcome>(json).unwrap(),
        outcome
    );
}

#[test]
fn given_a_failure_when_round_tripped_then_identical() {
    let outcome = CallOutcome::failure(
        report(),
        ClientError::RateLimited {
            retry_after: Some(Duration::from_secs(2)),
        },
    );
    let json = serde_json::to_value(&outcome).unwrap();
    assert_eq!(
        serde_json::from_value::<CallOutcome>(json).unwrap(),
        outcome
    );
}

#[test]
fn given_another_schema_when_an_outcome_is_loaded_then_refused() {
    let outcome = CallOutcome::failure(report(), ClientError::Transport);
    let mut json = serde_json::to_value(&outcome).unwrap();
    json["schema"] = serde_json::json!("br-llm-clients-core/2");
    assert!(serde_json::from_value::<CallOutcome>(json).is_err());
}

#[test]
fn given_a_stored_success_whose_usages_differ_when_loaded_then_refused() {
    let step = text_step("hi", Some(usage(10, 5)));
    let outcome = CallOutcome::success(report().with_usage(usage(10, 5)), step).unwrap();
    let mut json = serde_json::to_value(&outcome).unwrap();
    json["report"]["usage"]["output_tokens"] = serde_json::json!(6);
    assert!(serde_json::from_value::<CallOutcome>(json).is_err());
}

#[test]
fn given_a_stored_step_breaking_a_messages_rule_when_loaded_then_refused() {
    let json = serde_json::json!({
        "schema": "br-llm-clients-core/1",
        "report": { "total_duration": { "secs": 1, "nanos": 0 } },
        "result": {
            "status": "success",
            "step": { "content": [], "stop_reason": { "type": "end_turn" } }
        }
    });
    assert!(serde_json::from_value::<CallOutcome>(json).is_err());
}

#[test]
fn given_a_stored_tool_call_whose_name_br_llm_messages_refuses_when_loaded_then_refused() {
    let call = ToolCall {
        id: call_id("call-1"),
        name: ToolName::new("read_file").unwrap(),
        arguments: serde_json::json!({}),
    };
    let step = Step::new(
        vec![AssistantBlock::ToolCall(call)],
        StopReason::AwaitingToolResults,
        None,
        None,
    )
    .unwrap();
    let mut json = serde_json::to_value(CallOutcome::success(report(), step).unwrap()).unwrap();
    assert!(serde_json::from_value::<CallOutcome>(json.clone()).is_ok());
    json["result"]["step"]["content"][0]["name"] = serde_json::json!("fs.read");
    assert!(serde_json::from_value::<CallOutcome>(json).is_err());
}

#[test]
fn given_a_stored_error_case_unknown_to_this_version_when_loaded_then_refused() {
    let json = serde_json::json!({
        "schema": "br-llm-clients-core/1",
        "report": { "total_duration": { "secs": 1, "nanos": 0 } },
        "result": { "status": "failure", "error": { "type": "teapot" } }
    });
    assert!(serde_json::from_value::<CallOutcome>(json).is_err());
}

#[test]
fn given_a_stored_report_without_total_duration_when_loaded_then_refused() {
    let json = serde_json::json!({
        "schema": "br-llm-clients-core/1",
        "report": {},
        "result": { "status": "failure", "error": { "type": "transport" } }
    });
    assert!(serde_json::from_value::<CallOutcome>(json).is_err());
}

#[test]
fn given_an_outcome_when_taken_apart_then_report_and_result_come_out() {
    let outcome = CallOutcome::failure(report(), ClientError::Authentication);
    let (report, result) = outcome.into_parts();
    assert_eq!(report.total_duration(), Duration::from_millis(900));
    assert_eq!(result, Err(ClientError::Authentication));
}
