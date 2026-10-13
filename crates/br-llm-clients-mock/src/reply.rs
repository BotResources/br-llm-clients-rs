use br_llm_clients_core::ClientError;
use br_llm_messages::{Step, StreamEvent};

use crate::chunking::Chunking;
use crate::gate::Gate;
use crate::report_values::ReportValues;

#[derive(Debug, Clone)]
pub(crate) enum Shape {
    Success(Step),
    Failure(ClientError),
    Cut {
        step: Step,
        after: usize,
        error: ClientError,
    },
    Hang {
        step: Step,
        after: usize,
    },
    Broken {
        events: Vec<StreamEvent>,
        error: ClientError,
    },
}

#[derive(Debug, Clone)]
#[must_use = "a reply does nothing until it is given to a mock"]
pub struct Reply {
    pub(crate) shape: Shape,
    pub(crate) report: ReportValues,
    pub(crate) gates: Vec<(usize, Gate)>,
    pub(crate) chunking: Chunking,
    pub(crate) renaming: Option<String>,
}

impl Reply {
    pub(crate) fn step(&self) -> Option<&Step> {
        match &self.shape {
            Shape::Success(step) | Shape::Cut { step, .. } | Shape::Hang { step, .. } => Some(step),
            Shape::Failure(_) | Shape::Broken { .. } => None,
        }
    }
}
