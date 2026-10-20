use br_llm_clients_core::{CallOutcome, CallReport, ClientError};
use br_llm_messages::Step;

use crate::call_record::CallRecord;
use crate::error_clean::clean_error;
use crate::text_clean::Secrets;

pub(crate) fn outcome(
    secrets: &Secrets,
    record: CallRecord,
    result: Result<Step, ClientError>,
) -> CallOutcome {
    match result {
        Ok(step) => success(record, step, secrets),
        Err(error) => {
            let error = clean_error(error, secrets);
            let usage = record.usage.clone();
            CallOutcome::failure(report(&record, usage, secrets), error)
        }
    }
}

fn success(record: CallRecord, step: Step, secrets: &Secrets) -> CallOutcome {
    let report = report(&record, step.usage().cloned(), secrets);
    CallOutcome::success(report.clone(), step).unwrap_or_else(|_| {
        CallOutcome::failure(
            report,
            ClientError::MalformedResponse {
                detail: "the usage of the reply does not match the usage of its report".to_owned(),
            },
        )
    })
}

fn report(
    record: &CallRecord,
    usage: Option<br_llm_messages::Usage>,
    secrets: &Secrets,
) -> CallReport {
    let mut report = CallReport::new(record.started.elapsed());
    if let Some(usage) = usage {
        report = report.with_usage(usage);
    }
    if let Some(model) = record
        .model
        .clone()
        .or_else(|| record.answered_model.clone())
    {
        report = report.with_model(model);
    }
    if let Some(request_id) = record
        .request_id
        .clone()
        .filter(|id| !secrets.appears_in(id))
    {
        report = report.with_request_id(request_id);
    }
    if let Some(first) = record.time_to_first_event {
        report = report.with_time_to_first_event(first);
    }
    report
}
