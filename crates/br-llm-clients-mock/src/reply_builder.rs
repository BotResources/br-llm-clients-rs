use std::marker::PhantomData;
use std::time::Duration;

use br_llm_clients_core::ClientError;
use br_llm_messages::{ModelId, Step, StreamEvent, Usage};

use crate::build_error::MockBuildError;
use crate::chunking::Chunking;
use crate::gate::Gate;
use crate::reply::{Reply, Shape};
use crate::reply_check;
use crate::report_values::ReportValues;

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
pub struct ReplyBuilder<K: ReplyKind> {
    reply: Reply,
    kind: PhantomData<K>,
}

impl Reply {
    pub fn success(step: Step) -> ReplyBuilder<SuccessKind> {
        ReplyBuilder::start(Shape::Success(step))
    }

    pub fn failure(error: ClientError) -> ReplyBuilder<FailureKind> {
        ReplyBuilder::start(Shape::Failure(error))
    }

    pub fn cut(step: Step, after: usize, error: ClientError) -> ReplyBuilder<CutKind> {
        ReplyBuilder::start(Shape::Cut { step, after, error })
    }

    pub fn hang(step: Step, after: usize) -> ReplyBuilder<HangKind> {
        ReplyBuilder::start(Shape::Hang { step, after })
    }

    pub fn broken_stream(
        events: Vec<StreamEvent>,
        error: ClientError,
    ) -> ReplyBuilder<BrokenStreamKind> {
        ReplyBuilder::start(Shape::Broken { events, error })
    }
}

impl<K: ReplyKind> ReplyBuilder<K> {
    fn start(shape: Shape) -> Self {
        Self {
            reply: Reply {
                shape,
                report: ReportValues::default(),
                gates: Vec::new(),
                chunking: Chunking::none(),
                renaming: None,
            },
            kind: PhantomData,
        }
    }

    pub fn gate_at(mut self, position: usize, gate: &Gate) -> Self {
        self.reply.gates.push((position, gate.clone()));
        self
    }

    pub fn chunking(mut self, chunking: Chunking) -> Self {
        self.reply.chunking = chunking;
        self
    }

    pub fn rename_tool_call_ids(mut self, prefix: impl Into<String>) -> Self {
        self.reply.renaming = Some(prefix.into());
        self
    }

    pub fn build(self) -> Result<Reply, MockBuildError> {
        reply_check::check(&self.reply)?;
        Ok(self.reply)
    }
}

impl<K: Reported> ReplyBuilder<K> {
    pub fn model(mut self, model: ModelId) -> Self {
        self.reply.report.model = Some(model);
        self
    }

    pub fn request_id(mut self, request_id: impl Into<String>) -> Self {
        self.reply.report.request_id = Some(request_id.into());
        self
    }

    pub fn time_to_first_event(mut self, time: Duration) -> Self {
        self.reply.report.time_to_first_event = Some(time);
        self
    }

    pub fn total_duration(mut self, duration: Duration) -> Self {
        self.reply.report.total_duration = Some(duration);
        self
    }
}

impl<K: NamesUsage> ReplyBuilder<K> {
    pub fn usage(mut self, usage: Usage) -> Self {
        self.reply.report.usage = Some(usage);
        self
    }
}
