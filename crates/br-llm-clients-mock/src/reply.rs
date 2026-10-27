use br_llm_clients_core::ClientError;
use br_llm_messages::{ModelResponse, ResponseDelta};

use crate::build_error::MockBuildError;
use crate::chunking::Chunking;
use crate::gate::Gate;
use crate::report_values::ReportValues;
use crate::response_input::ResponseInput;

#[derive(Debug, Clone)]
pub(crate) enum Shape<R> {
    Success(R),
    Failure(ClientError),
    Cut {
        response: R,
        after: usize,
        error: ClientError,
    },
    Hang {
        response: R,
        after: usize,
    },
    Broken {
        deltas: Vec<ResponseDelta>,
        error: ClientError,
    },
}

impl Shape<ResponseInput> {
    pub(crate) fn resolve(self) -> Result<Shape<ModelResponse>, MockBuildError> {
        Ok(match self {
            Shape::Success(input) => Shape::Success(input.resolve()?),
            Shape::Failure(error) => Shape::Failure(error),
            Shape::Cut {
                response,
                after,
                error,
            } => Shape::Cut {
                response: response.resolve()?,
                after,
                error,
            },
            Shape::Hang { response, after } => Shape::Hang {
                response: response.resolve()?,
                after,
            },
            Shape::Broken { deltas, error } => Shape::Broken { deltas, error },
        })
    }
}

#[derive(Debug, Clone)]
#[must_use = "a reply does nothing until it is given to a mock"]
pub struct Reply {
    pub(crate) shape: Shape<ModelResponse>,
    pub(crate) report: ReportValues,
    pub(crate) gates: Vec<(usize, Gate)>,
    pub(crate) chunking: Chunking,
    pub(crate) request_identities: bool,
}

impl Reply {
    pub(crate) fn response(&self) -> Option<&ModelResponse> {
        match &self.shape {
            Shape::Success(response)
            | Shape::Cut { response, .. }
            | Shape::Hang { response, .. } => Some(response),
            Shape::Failure(_) | Shape::Broken { .. } => None,
        }
    }
}
