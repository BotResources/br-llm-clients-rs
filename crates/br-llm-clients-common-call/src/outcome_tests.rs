use std::time::Duration;

use br_llm_clients_common_http::Secret;
use br_llm_clients_core::ClientError;
use br_llm_messages::{AssistantBlock, ModelId, Step, StopReason, Text, Usage};
use tokio::time::advance;

use crate::call_record::CallRecord;
use crate::outcome::outcome;
use crate::text_clean::Secrets;

fn usage(input: u64) -> Usage {
    Usage {
        input_tokens: input,
        output_tokens: 2,
        ..Usage::default()
    }
}

fn step(usage: Option<Usage>) -> Step {
    let text = Text::new("hi").expect("text");
    Step::new(
        vec![AssistantBlock::Text { text }],
        StopReason::EndTurn,
        usage,
        None,
    )
    .expect("step")
}

fn record() -> CallRecord {
    let mut record = CallRecord::start();
    record.request_id = Some("req-1".to_owned());
    record.model = Some(ModelId::new("answering-model").expect("model"));
    record.time_to_first_event = Some(Duration::from_millis(30));
    record
}

#[tokio::test(start_paused = true)]
async fn given_a_step_when_the_outcome_is_built_then_the_report_holds_the_usage_of_the_step_and_the_record()
 {
    let record = CallRecord {
        usage: Some(usage(99)),
        ..record()
    };
    advance(Duration::from_secs(4)).await;
    let outcome = outcome(&Secrets::new(Vec::new()), record, Ok(step(Some(usage(7)))));
    let (report, result) = outcome.into_parts();
    assert_eq!(result, Ok(step(Some(usage(7)))));
    assert_eq!(report.usage(), Some(&usage(7)));
    assert_eq!(report.model().map(ModelId::as_str), Some("answering-model"));
    assert_eq!(report.request_id(), Some("req-1"));
    assert_eq!(
        report.time_to_first_event(),
        Some(Duration::from_millis(30))
    );
    assert_eq!(report.total_duration(), Duration::from_secs(4));
}

#[tokio::test(start_paused = true)]
async fn given_a_step_without_usage_when_the_outcome_is_built_then_the_report_holds_none() {
    let record = CallRecord {
        usage: Some(usage(99)),
        ..record()
    };
    let (report, result) = outcome(&Secrets::new(Vec::new()), record, Ok(step(None))).into_parts();
    assert!(result.is_ok());
    assert_eq!(report.usage(), None);
}

#[tokio::test(start_paused = true)]
async fn given_a_failure_when_the_outcome_is_built_then_the_report_holds_the_last_usage_and_the_record()
 {
    let record = CallRecord {
        usage: Some(usage(5)),
        ..record()
    };
    advance(Duration::from_secs(3)).await;
    let (report, result) = outcome(
        &Secrets::new(Vec::new()),
        record,
        Err(ClientError::StreamTruncated),
    )
    .into_parts();
    assert_eq!(result, Err(ClientError::StreamTruncated));
    assert_eq!(report.usage(), Some(&usage(5)));
    assert_eq!(report.model().map(ModelId::as_str), Some("answering-model"));
    assert_eq!(report.request_id(), Some("req-1"));
    assert_eq!(
        report.time_to_first_event(),
        Some(Duration::from_millis(30))
    );
    assert_eq!(report.total_duration(), Duration::from_secs(3));
}

#[tokio::test(start_paused = true)]
async fn given_a_failure_without_usage_when_the_outcome_is_built_then_the_report_holds_none() {
    let (report, _) = outcome(
        &Secrets::new(Vec::new()),
        record(),
        Err(ClientError::StreamTruncated),
    )
    .into_parts();
    assert_eq!(report.usage(), None);
}

#[tokio::test(start_paused = true)]
async fn given_a_failure_with_a_secret_when_the_outcome_is_built_then_the_error_is_cleaned() {
    let secrets = Secrets::new(vec![Secret::new("TOPSECRET").expect("secret")]);
    let error = ClientError::MalformedResponse {
        detail: "TOPSECRET".to_owned(),
    };
    let (_, result) = outcome(&secrets, record(), Err(error)).into_parts();
    assert_eq!(
        result,
        Err(ClientError::MalformedResponse {
            detail: "[redacted]".to_owned()
        })
    );
}
