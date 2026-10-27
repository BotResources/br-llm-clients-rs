use std::future::Future;
use std::task::{Context, Poll, Waker};

use br_llm_messages::{
    ContentBlock, ModelResponse, PlainText, ResponseEnding, ResponseMetadata, ToolName, Usage,
};

use crate::thread_fixtures::{block_id, entry_id};
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

pub(crate) fn plain_text(value: &str) -> PlainText {
    PlainText::new(value).unwrap()
}

pub(crate) fn text_response(value: &str, usage: Option<Usage>) -> ModelResponse {
    let block = br_llm_messages::TextBlock::new(block_id(), value).unwrap();
    let metadata = match usage {
        Some(usage) => ResponseMetadata::new().with_usage(usage),
        None => ResponseMetadata::new(),
    };
    ModelResponse::new(
        entry_id(),
        vec![ContentBlock::Text(block)],
        ResponseEnding::finished(),
        metadata,
    )
    .unwrap()
}

pub(crate) fn usage(input: u64, output: u64) -> Usage {
    Usage::of_input_tokens(input).with_output_tokens(output)
}
