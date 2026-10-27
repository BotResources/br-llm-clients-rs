use br_llm_clients_core::{CallOutcome, CallReport, ClientError};
use br_llm_messages::ModelResponse;

use crate::report_values::bare_report;

pub(crate) fn inconsistent(cause: &impl std::fmt::Display) -> ClientError {
    ClientError::MalformedResponse {
        detail: format!("the mock built an inconsistent reply: {cause}"),
    }
}

pub(crate) fn refused(error: ClientError) -> CallOutcome {
    CallOutcome::failure(bare_report(), error)
}

pub(crate) fn failed(report: CallReport, error: ClientError) -> CallOutcome {
    CallOutcome::failure(report, error)
}

pub(crate) fn succeeded(report: CallReport, response: ModelResponse) -> CallOutcome {
    match CallOutcome::success(report, response) {
        Ok(outcome) => outcome,
        Err(error) => refused(inconsistent(&error)),
    }
}
