use std::marker::PhantomData;
use std::time::Duration;

use br_llm_clients_core::ClientError;
use br_llm_messages::{ModelName, ResponseDelta, Usage};

use crate::build_error::MockBuildError;
use crate::chunking::Chunking;
use crate::gate::Gate;
use crate::json_input::JsonInput;
use crate::reply::{Reply, Shape};
use crate::reply_check;
use crate::report_values::ReportValues;
use crate::response_input::ResponseInput;
use crate::sketch::Sketch;

mod sealed {
    pub trait Sealed {}
}

pub trait ReplyKind: sealed::Sealed {}

pub trait Reported: ReplyKind {}

pub trait NamesUsage: Reported {}

#[derive(Debug, Clone, Copy)]
pub struct SuccessKind;
#[derive(Debug, Clone, Copy)]
pub struct FailureKind;
#[derive(Debug, Clone, Copy)]
pub struct CutKind;
#[derive(Debug, Clone, Copy)]
pub struct HangKind;
#[derive(Debug, Clone, Copy)]
pub struct BrokenStreamKind;

macro_rules! kinds {
    ($($kind:ident),*) => {
        $(impl sealed::Sealed for $kind {} impl ReplyKind for $kind {})*
    };
}

kinds!(
    SuccessKind,
    FailureKind,
    CutKind,
    HangKind,
    BrokenStreamKind
);

impl Reported for SuccessKind {}
impl Reported for FailureKind {}
impl Reported for CutKind {}
impl Reported for BrokenStreamKind {}
impl NamesUsage for FailureKind {}
impl NamesUsage for CutKind {}
impl NamesUsage for BrokenStreamKind {}

#[derive(Debug, Clone)]
#[must_use]
pub struct ReplyBuilder<K: ReplyKind> {
    shape: Shape<ResponseInput>,
    report: ReportValues,
    gates: Vec<(usize, Gate)>,
    chunking: Chunking,
    request_identities: bool,
    kind: PhantomData<K>,
}

impl Reply {
    pub fn success(response: impl Into<ResponseInput>) -> ReplyBuilder<SuccessKind> {
        ReplyBuilder::start(Shape::Success(response.into()))
    }

    pub fn text(text: impl Into<String>) -> ReplyBuilder<SuccessKind> {
        Reply::success(Sketch::new().text(text))
    }

    pub fn tool_call(
        name: impl Into<String>,
        arguments: impl Into<JsonInput>,
    ) -> ReplyBuilder<SuccessKind> {
        Reply::success(Sketch::new().tool_call(name, arguments))
    }

    pub fn structured(value: impl Into<JsonInput>) -> ReplyBuilder<SuccessKind> {
        Reply::success(Sketch::new().structured(value))
    }

    pub fn failure(error: ClientError) -> ReplyBuilder<FailureKind> {
        ReplyBuilder::start(Shape::Failure(error))
    }

    pub fn cut(
        response: impl Into<ResponseInput>,
        after: usize,
        error: ClientError,
    ) -> ReplyBuilder<CutKind> {
        ReplyBuilder::start(Shape::Cut {
            response: response.into(),
            after,
            error,
        })
    }

    pub fn hang(response: impl Into<ResponseInput>, after: usize) -> ReplyBuilder<HangKind> {
        ReplyBuilder::start(Shape::Hang {
            response: response.into(),
            after,
        })
    }

    pub fn broken_stream(
        deltas: Vec<ResponseDelta>,
        error: ClientError,
    ) -> ReplyBuilder<BrokenStreamKind> {
        ReplyBuilder::start(Shape::Broken { deltas, error })
    }
}

impl<K: ReplyKind> ReplyBuilder<K> {
    fn start(shape: Shape<ResponseInput>) -> Self {
        Self {
            shape,
            report: ReportValues::default(),
            gates: Vec::new(),
            chunking: Chunking::none(),
            request_identities: false,
            kind: PhantomData,
        }
    }

    pub fn gate_at(mut self, position: usize, gate: &Gate) -> Self {
        self.gates.push((position, gate.clone()));
        self
    }

    pub fn chunking(mut self, chunking: Chunking) -> Self {
        self.chunking = chunking;
        self
    }

    pub fn build(self) -> Result<Reply, MockBuildError> {
        let reply = Reply {
            shape: self.shape.resolve()?,
            report: self.report,
            gates: self.gates,
            chunking: self.chunking,
            request_identities: self.request_identities,
        };
        reply_check::check(&reply)?;
        Ok(reply)
    }
}

impl ReplyBuilder<BrokenStreamKind> {
    pub fn with_request_identities(mut self) -> Self {
        self.request_identities = true;
        self
    }
}

impl<K: Reported> ReplyBuilder<K> {
    pub fn request_id(mut self, request_id: impl Into<String>) -> Self {
        self.report.request_id = Some(request_id.into());
        self
    }

    pub fn time_to_first_event(mut self, time: Duration) -> Self {
        self.report.time_to_first_event = Some(time);
        self
    }

    pub fn total_duration(mut self, duration: Duration) -> Self {
        self.report.total_duration = Some(duration);
        self
    }
}

impl<K: NamesUsage> ReplyBuilder<K> {
    pub fn usage(mut self, usage: Usage) -> Self {
        self.report.usage = Some(usage);
        self
    }

    pub fn model(mut self, model: ModelName) -> Self {
        self.report.model = Some(model);
        self
    }
}
