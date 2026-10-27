use std::time::Duration;

use br_llm_messages::ModelName;

use crate::build_error::BuildError;
use crate::test_support::{text_response, usage};
use crate::{CallOutcome, CallReport, ClientError};

fn report() -> CallReport {
    CallReport::new(Duration::from_millis(900))
}

#[test]
fn given_a_report_with_the_usage_of_the_response_when_a_success_is_built_then_accepted() {
    let response = text_response("hi", Some(usage(10, 5)));
    let outcome =
        CallOutcome::success(report().with_usage(usage(10, 5)), response.clone()).unwrap();
    assert_eq!(outcome.result(), Ok(&response));
}

#[test]
fn given_a_report_and_a_response_both_without_usage_when_a_success_is_built_then_accepted() {
    assert!(CallOutcome::success(report(), text_response("hi", None)).is_ok());
}

#[test]
fn given_a_report_usage_differing_from_the_response_when_a_success_is_built_then_refused() {
    let response = text_response("hi", Some(usage(10, 5)));
    assert_eq!(
        CallOutcome::success(report().with_usage(usage(10, 6)), response.clone()),
        Err(BuildError::UsageMismatch)
    );
}

#[test]
fn given_a_usage_on_one_side_only_when_a_success_is_built_then_refused_both_ways() {
    let with_usage = text_response("hi", Some(usage(10, 5)));
    assert_eq!(
        CallOutcome::success(report(), with_usage),
        Err(BuildError::UsageMismatch)
    );
    assert_eq!(
        CallOutcome::success(report().with_usage(usage(10, 5)), text_response("hi", None)),
        Err(BuildError::UsageMismatch)
    );
}

#[test]
fn given_a_usage_with_other_counts_when_a_success_is_built_then_refused() {
    let response = text_response("hi", Some(usage(10, 5)));
    let thinking = usage(10, 5).with_thinking_tokens(2);
    assert_eq!(
        CallOutcome::success(report().with_usage(thinking), response),
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
fn given_a_full_report_when_read_then_each_item_comes_back() {
    let report = report()
        .with_usage(usage(1, 2))
        .with_model(ModelName::new("m").unwrap())
        .with_request_id("req-1")
        .with_time_to_first_event(Duration::from_millis(120));
    assert_eq!(report.total_duration(), Duration::from_millis(900));
    assert_eq!(
        report.time_to_first_event(),
        Some(Duration::from_millis(120))
    );
    assert_eq!(report.request_id(), Some("req-1"));
    assert_eq!(report.model().map(ModelName::as_str), Some("m"));
    assert_eq!(report.usage(), Some(&usage(1, 2)));
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
fn given_an_outcome_when_taken_apart_then_report_and_result_come_out() {
    let outcome = CallOutcome::failure(report(), ClientError::Authentication);
    let (report, result) = outcome.into_parts();
    assert_eq!(report.total_duration(), Duration::from_millis(900));
    assert_eq!(result, Err(ClientError::Authentication));
}
