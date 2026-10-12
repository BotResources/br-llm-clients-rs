use br_llm_messages::Step;

use crate::build_error::BuildError;
use crate::call_report::CallReport;
use crate::client_error::ClientError;
use crate::schema::{self, LoadError};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "StoredCallOutcome", into = "StoredCallOutcome")]
pub struct CallOutcome {
    report: CallReport,
    result: Result<Step, ClientError>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredCallOutcome {
    schema: String,
    report: CallReport,
    result: StoredResult,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
enum StoredResult {
    Success { step: Step },
    Failure { error: ClientError },
}

impl TryFrom<StoredCallOutcome> for CallOutcome {
    type Error = LoadError;

    fn try_from(stored: StoredCallOutcome) -> Result<Self, Self::Error> {
        schema::check(&stored.schema)?;
        match stored.result {
            StoredResult::Success { step } => Ok(CallOutcome::success(stored.report, step)?),
            StoredResult::Failure { error } => Ok(CallOutcome::failure(stored.report, error)),
        }
    }
}

impl From<CallOutcome> for StoredCallOutcome {
    fn from(outcome: CallOutcome) -> Self {
        let result = match outcome.result {
            Ok(step) => StoredResult::Success { step },
            Err(error) => StoredResult::Failure { error },
        };
        Self {
            schema: schema::current(),
            report: outcome.report,
            result,
        }
    }
}

impl CallOutcome {
    pub fn success(report: CallReport, step: Step) -> Result<Self, BuildError> {
        if report.usage() != step.usage() {
            return Err(BuildError::UsageMismatch);
        }
        Ok(Self {
            report,
            result: Ok(step),
        })
    }

    pub fn failure(report: CallReport, error: ClientError) -> Self {
        Self {
            report,
            result: Err(error),
        }
    }

    pub fn report(&self) -> &CallReport {
        &self.report
    }

    pub fn result(&self) -> Result<&Step, &ClientError> {
        self.result.as_ref()
    }

    pub fn into_parts(self) -> (CallReport, Result<Step, ClientError>) {
        (self.report, self.result)
    }
}
