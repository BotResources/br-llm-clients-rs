use br_llm_messages::ModelResponse;

use crate::build_error::BuildError;
use crate::call_report::CallReport;
use crate::client_error::ClientError;

#[derive(Debug, Clone, PartialEq)]
pub struct CallOutcome {
    pub(crate) report: CallReport,
    pub(crate) result: Result<ModelResponse, ClientError>,
}

impl CallOutcome {
    pub fn success(report: CallReport, response: ModelResponse) -> Result<Self, BuildError> {
        if report.usage() != response.usage() {
            return Err(BuildError::UsageMismatch);
        }
        Ok(Self {
            report,
            result: Ok(response),
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

    pub fn result(&self) -> Result<&ModelResponse, &ClientError> {
        self.result.as_ref()
    }

    pub fn into_parts(self) -> (CallReport, Result<ModelResponse, ClientError>) {
        (self.report, self.result)
    }
}
