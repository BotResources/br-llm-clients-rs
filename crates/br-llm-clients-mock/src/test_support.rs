use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Wake, Waker};
use std::thread::Thread;

use br_llm_clients_core::{
    CacheRequirement, CallOutcome, Capabilities, CapabilitiesSpec, Client, ClientError,
    ClientRequest, ClientSettings, MaxOutputTokens, OutputMode, Parameters, ReasoningLevel,
    StructuredShape, ToolSpec,
};
use br_llm_messages::{
    ContentBlock, DeltaOp, MediaType, ModelName, ModelResponse, OpaqueJson, PlainBlock,
    ResponseDelta, ResponseEnding, ResponseMetadata, TextBlock, ToolName, Usage,
};

use crate::chunking::Chunking;
use crate::derive_deltas::derive_deltas;
use crate::fixtures::{
    agent_id, answered_turn, block_id, entry_id, human_turn, project, text_block, thread_of,
    turn_id, unanswered_call_turn,
};
use crate::mock_client::MockClient;
use crate::recording_sink::RecordingSink;
use crate::renew::renew;
use crate::reply_source::ReplySource;
use crate::rule::Rule;
use crate::sequential_id_source::SequentialIdSource;

struct Unparker(Thread);

impl Wake for Unparker {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

pub(crate) fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::from(Arc::new(Unparker(std::thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::park(),
        }
    }
}

pub(crate) fn poll_once<F: Future + ?Sized>(future: &mut std::pin::Pin<&mut F>) -> Poll<F::Output> {
    future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
}

#[derive(Default)]
pub(crate) struct WakeCount(AtomicUsize);

impl WakeCount {
    pub(crate) fn waker(self: &Arc<Self>) -> Waker {
        Waker::from(Arc::clone(self))
    }

    pub(crate) fn count(&self) -> usize {
        self.0.load(Ordering::SeqCst)
    }
}

impl Wake for WakeCount {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

pub(crate) fn poll_with<F: Future + ?Sized>(
    future: &mut std::pin::Pin<&mut F>,
    waker: &Waker,
) -> Poll<F::Output> {
    future.as_mut().poll(&mut Context::from_waker(waker))
}

pub(crate) fn model(name: &str) -> ModelName {
    ModelName::new(name).unwrap()
}

pub(crate) fn settings() -> ClientSettings {
    settings_with(ReasoningLevel::Medium, CacheRequirement::NotRequired, 1000)
}

pub(crate) fn settings_with(
    level: ReasoningLevel,
    cache: CacheRequirement,
    tokens: u32,
) -> ClientSettings {
    ClientSettings::new(
        model("settings-model"),
        level,
        cache,
        MaxOutputTokens::new(tokens).unwrap(),
    )
}

pub(crate) fn usage(input: u64, output: u64) -> Usage {
    Usage::of_input_tokens(input).with_output_tokens(output)
}

pub(crate) fn response_of(
    blocks: Vec<ContentBlock>,
    ending: ResponseEnding,
    metadata: ResponseMetadata,
) -> ModelResponse {
    ModelResponse::new(entry_id(), blocks, ending, metadata).unwrap()
}

pub(crate) fn text_content(value: &str) -> ContentBlock {
    ContentBlock::Text(TextBlock::new(block_id(), value).unwrap())
}

pub(crate) fn text_response(value: &str) -> ModelResponse {
    response_of(
        vec![text_content(value)],
        ResponseEnding::finished(),
        ResponseMetadata::new(),
    )
}

pub(crate) fn text_response_with_usage(value: &str, usage: Usage) -> ModelResponse {
    response_of(
        vec![text_content(value)],
        ResponseEnding::finished(),
        ResponseMetadata::new().with_usage(usage),
    )
}

pub(crate) fn call_content(name: &str, arguments: &str) -> ContentBlock {
    ContentBlock::RuntimeToolCall(
        br_llm_messages::RuntimeToolCall::new(
            block_id(),
            ToolName::new(name).unwrap(),
            OpaqueJson::parse(arguments).unwrap(),
        )
        .unwrap(),
    )
}

pub(crate) fn calls_response(count: usize) -> ModelResponse {
    response_of(
        (0..count).map(|_| call_content("search", "{}")).collect(),
        ResponseEnding::tool_use(),
        ResponseMetadata::new(),
    )
}

pub(crate) fn delta(op: DeltaOp) -> ResponseDelta {
    ResponseDelta::new(turn_id(), entry_id(), op)
}

pub(crate) fn started_delta() -> ResponseDelta {
    delta(DeltaOp::response_started(ResponseMetadata::new()))
}

pub(crate) fn tool() -> ToolSpec {
    ToolSpec::new(ToolName::new("search").unwrap(), "", Parameters::default()).unwrap()
}

pub(crate) fn shape() -> OutputMode {
    OutputMode::Structured(StructuredShape::new("answer", None, Parameters::default()).unwrap())
}

pub(crate) fn request_of(
    content: Vec<PlainBlock>,
    tools: Vec<ToolSpec>,
    output: OutputMode,
) -> ClientRequest {
    ClientRequest::new(
        None,
        thread_of(vec![human_turn(content)]),
        turn_id(),
        entry_id(),
        tools,
        output,
    )
    .unwrap()
}

pub(crate) fn request() -> ClientRequest {
    crate::fixtures::with_fixed_ids(900_000, || {
        request_of(vec![text_block("hello")], vec![], OutputMode::Plain)
    })
}

pub(crate) fn long_request() -> ClientRequest {
    crate::fixtures::with_fixed_ids(800_000, long_request_body)
}

fn long_request_body() -> ClientRequest {
    let agent = agent_id();
    let turns = vec![
        human_turn(vec![text_block("a")]),
        answered_turn(agent, "b"),
        human_turn(vec![text_block("c")]),
    ];
    ClientRequest::new(
        None,
        project(agent, turns),
        turn_id(),
        entry_id(),
        vec![],
        OutputMode::Plain,
    )
    .unwrap()
}

pub(crate) fn bad_request() -> ClientRequest {
    let agent = agent_id();
    ClientRequest::new(
        None,
        project(agent, vec![unanswered_call_turn(agent)]),
        turn_id(),
        entry_id(),
        vec![],
        OutputMode::Plain,
    )
    .unwrap()
}

pub(crate) fn image_request(media_type: MediaType) -> ClientRequest {
    request_of(
        vec![text_block("look"), crate::fixtures::plain_image(media_type)],
        vec![],
        OutputMode::Plain,
    )
}

pub(crate) fn everything_caps() -> Capabilities {
    Capabilities::new(CapabilitiesSpec::everything()).unwrap()
}

pub(crate) fn always_rule(reply: impl Into<ReplySource>) -> Rule {
    Rule::new(|_| true, reply)
}

pub(crate) fn mock() -> MockClient {
    MockClient::new(settings(), everything_caps())
}

pub(crate) fn run(client: &MockClient, request: &ClientRequest) -> (CallOutcome, RecordingSink) {
    let mut sink = RecordingSink::new();
    let outcome = block_on(client.call(request, &mut sink));
    (outcome, sink)
}

pub(crate) fn error_of(outcome: &CallOutcome) -> ClientError {
    outcome.result().unwrap_err().clone()
}

pub(crate) fn response_from(outcome: &CallOutcome) -> ModelResponse {
    outcome.result().unwrap().clone()
}

pub(crate) fn text_of(outcome: &CallOutcome) -> String {
    response_from(outcome)
        .blocks()
        .iter()
        .find_map(|block| match block {
            ContentBlock::Text(text) => Some(text.text().as_str().to_owned()),
            _ => None,
        })
        .unwrap()
}

pub(crate) fn caps(change: impl FnOnce(&mut CapabilitiesSpec)) -> Capabilities {
    let mut spec = CapabilitiesSpec::everything();
    change(&mut spec);
    Capabilities::new(spec).unwrap()
}

pub(crate) fn expected_deltas(
    request: &ClientRequest,
    response: &ModelResponse,
    chunking: &Chunking,
) -> Vec<ResponseDelta> {
    let renewed = renew(
        &SequentialIdSource::of_mock(),
        request.reply_entry(),
        response,
    )
    .unwrap();
    derive_deltas(request.reply_turn(), &renewed, chunking)
        .unwrap()
        .into_deltas()
}
