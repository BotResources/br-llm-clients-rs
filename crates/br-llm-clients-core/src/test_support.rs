use std::future::Future;
use std::task::{Context, Poll, Waker};

use br_llm_messages::{AssistantBlock, Step, StopReason, Text, ToolCallId, ToolName, Usage};

use crate::{Parameter, ParameterKind, Parameters, ToolSpec};

pub(crate) fn block_on<F: Future>(future: F) -> F::Output {
    let mut context = Context::from_waker(Waker::noop());
    let mut future = std::pin::pin!(future);
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return output;
        }
    }
}

pub(crate) fn scalar(name: &str) -> Parameter {
    Parameter::new(name, "", true, ParameterKind::String).unwrap()
}

pub(crate) fn parameters(list: Vec<Parameter>) -> Parameters {
    Parameters::new(list).unwrap()
}

pub(crate) fn tool(name: &str) -> ToolSpec {
    ToolSpec::new(ToolName::new(name).unwrap(), "", Parameters::default()).unwrap()
}

pub(crate) fn text(value: &str) -> Text {
    Text::new(value).unwrap()
}

pub(crate) fn call_id(value: &str) -> ToolCallId {
    ToolCallId::new(value).unwrap()
}

pub(crate) fn text_step(value: &str, usage: Option<Usage>) -> Step {
    Step::new(
        vec![AssistantBlock::Text { text: text(value) }],
        StopReason::EndTurn,
        usage,
        None,
    )
    .unwrap()
}

pub(crate) fn usage(input: u64, output: u64) -> Usage {
    Usage {
        input_tokens: input,
        output_tokens: output,
        ..Usage::default()
    }
}
