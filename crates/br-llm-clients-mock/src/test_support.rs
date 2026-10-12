use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Wake, Waker};
use std::thread::Thread;

use br_llm_clients_core::{
    CacheRequirement, CallOutcome, Client, ClientError, ClientRequest, ClientSettings,
    MaxOutputTokens, OutputMode, ReasoningLevel, StructuredShape,
};
use br_llm_clients_core::{Parameters, ToolSpec};
use br_llm_messages::{
    AssistantBlock, Base64Data, Image, ImageMime, ModelId, Step, StopReason, StreamEvent, Text,
    ToolCall, ToolCallId, ToolName, ToolResult, ToolResultBlock, Usage, WireMessage, WireUserBlock,
};

use crate::capabilities::{Capabilities, CapabilitiesSpec};
use crate::mock_client::MockClient;
use crate::recording_sink::RecordingSink;

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

pub(crate) fn model(name: &str) -> ModelId {
    ModelId::new(name).unwrap()
}

pub(crate) fn text(value: &str) -> Text {
    Text::new(value).unwrap()
}

pub(crate) fn call_id(value: &str) -> ToolCallId {
    ToolCallId::new(value).unwrap()
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
    Usage {
        input_tokens: input,
        output_tokens: output,
        ..Usage::default()
    }
}

pub(crate) fn text_step(value: &str) -> Step {
    step(
        vec![AssistantBlock::Text { text: text(value) }],
        StopReason::EndTurn,
    )
}

pub(crate) fn step(content: Vec<AssistantBlock>, stop_reason: StopReason) -> Step {
    Step::new(content, stop_reason, None, None).unwrap()
}

pub(crate) fn step_with_usage(value: &str, usage: Usage) -> Step {
    Step::new(
        vec![AssistantBlock::Text { text: text(value) }],
        StopReason::EndTurn,
        Some(usage),
        None,
    )
    .unwrap()
}

pub(crate) fn tool_call(id: &str, name: &str, arguments: serde_json::Value) -> AssistantBlock {
    AssistantBlock::ToolCall(ToolCall {
        id: call_id(id),
        name: ToolName::new(name).unwrap(),
        arguments,
    })
}

pub(crate) fn calls_step(ids: &[&str]) -> Step {
    let content = ids
        .iter()
        .map(|id| tool_call(id, "search", serde_json::json!({ "q": id })))
        .collect();
    step(content, StopReason::AwaitingToolResults)
}

pub(crate) fn user_text(value: &str) -> WireMessage {
    WireMessage::User {
        content: vec![WireUserBlock::Text { text: text(value) }],
    }
}

pub(crate) fn user_image(mime: ImageMime) -> WireMessage {
    WireMessage::User {
        content: vec![WireUserBlock::Image(Image {
            mime,
            data: Base64Data::new("aGVsbG8=").unwrap(),
        })],
    }
}

pub(crate) fn user_result(id: &str) -> WireMessage {
    WireMessage::User {
        content: vec![WireUserBlock::ToolResult(ToolResult::new(
            call_id(id),
            ToolName::new("search").unwrap(),
            vec![ToolResultBlock::text(text("found"))],
            false,
        ))],
    }
}

pub(crate) fn assistant_text(value: &str) -> WireMessage {
    WireMessage::Assistant {
        content: vec![AssistantBlock::Text { text: text(value) }],
    }
}

pub(crate) fn assistant_call(id: &str) -> WireMessage {
    WireMessage::Assistant {
        content: vec![tool_call(id, "search", serde_json::json!({}))],
    }
}

pub(crate) fn tool() -> ToolSpec {
    ToolSpec::new(ToolName::new("search").unwrap(), "", Parameters::default()).unwrap()
}

pub(crate) fn shape() -> OutputMode {
    OutputMode::Structured(StructuredShape::new("answer", None, Parameters::default()).unwrap())
}

pub(crate) fn request_of(
    messages: Vec<WireMessage>,
    tools: Vec<ToolSpec>,
    output: OutputMode,
) -> ClientRequest {
    ClientRequest::new(None, messages, tools, output).unwrap()
}

pub(crate) fn request() -> ClientRequest {
    request_of(vec![user_text("hello")], vec![], OutputMode::Plain)
}

pub(crate) fn mock() -> MockClient {
    MockClient::new(settings(), Capabilities::everything())
}

pub(crate) fn run(client: &MockClient, request: &ClientRequest) -> (CallOutcome, RecordingSink) {
    let mut sink = RecordingSink::new();
    let outcome = block_on(client.call(request, &mut sink));
    (outcome, sink)
}

pub(crate) fn error_of(outcome: &CallOutcome) -> ClientError {
    outcome.result().unwrap_err().clone()
}

pub(crate) fn step_of(outcome: &CallOutcome) -> Step {
    outcome.result().unwrap().clone()
}

pub(crate) fn caps(change: impl FnOnce(&mut CapabilitiesSpec)) -> Capabilities {
    let mut spec = CapabilitiesSpec::everything();
    change(&mut spec);
    Capabilities::new(spec).unwrap()
}

pub(crate) fn is_unknown_value(error: &ClientError, type_name: &str) -> bool {
    matches!(error, ClientError::ProviderRejected { status: None, code: Some(code), body }
        if code == crate::codes::CODE_UNKNOWN_VALUE && body.contains(type_name))
}

pub(crate) fn finish_event() -> StreamEvent {
    StreamEvent::Finish {
        stop_reason: StopReason::EndTurn,
        usage: None,
    }
}
