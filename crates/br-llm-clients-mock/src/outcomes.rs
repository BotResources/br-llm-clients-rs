use br_llm_clients_core::{CallOutcome, CallReport, ClientError};
use br_llm_messages::Step;

use crate::codes;
use crate::report_values::bare_report;

pub(crate) fn refused(error: ClientError) -> CallOutcome {
    CallOutcome::failure(bare_report(), error)
}

pub(crate) fn failed(report: CallReport, error: ClientError) -> CallOutcome {
    CallOutcome::failure(report, error)
}

pub(crate) fn succeeded(report: CallReport, step: Step) -> CallOutcome {
    match CallOutcome::success(report, step) {
        Ok(outcome) => outcome,
        Err(error) => refused(codes::invalid_reply("the reply", &error)),
    }
}
