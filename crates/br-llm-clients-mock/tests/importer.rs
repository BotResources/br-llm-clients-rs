use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::thread::Thread;

use br_llm_clients_core::{
    CacheRequirement, Client, ClientError, ClientRequest, ClientSettings, MaxOutputTokens,
    OutputMode, ReasoningLevel,
};
use br_llm_clients_mock::{
    Accepted, Capabilities, CapabilitiesSpec, Chunking, Gate, MockClient, RecordingSink, Reply,
    ReplySource, Rule, StateKind, derive_events, tool_call_id,
};
use br_llm_messages::{
    AssistantBlock, ModelId, Step, StopReason, Text, ToolCall, ToolName, WireMessage, WireUserBlock,
};

struct Unparker(Thread);

impl Wake for Unparker {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

fn block_on<F: Future>(future: F) -> F::Output {
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

fn settings() -> ClientSettings {
    ClientSettings::new(
        ModelId::new("test-model").unwrap(),
        ReasoningLevel::Medium,
        CacheRequirement::NotRequired,
        MaxOutputTokens::new(512).unwrap(),
    )
}

fn request() -> ClientRequest {
    let message = WireMessage::User {
        content: vec![WireUserBlock::Text {
            text: Text::new("hello").unwrap(),
        }],
    };
    ClientRequest::new(None, vec![message], vec![], OutputMode::Plain).unwrap()
}

fn answer(value: &str) -> Step {
    Step::new(
        vec![AssistantBlock::Text {
            text: Text::new(value).unwrap(),
        }],
        StopReason::EndTurn,
        None,
        None,
    )
    .unwrap()
}

#[test]
fn given_the_public_surface_when_a_test_writes_a_script_then_the_code_under_test_gets_the_answers()
{
    let client = MockClient::builder(settings(), Capabilities::everything())
        .reply(Reply::success(answer("scripted")).build().unwrap())
        .build();
    let shared: Arc<dyn Client> = Arc::new(client.clone());
    let mut sink = RecordingSink::new();
    let outcome = block_on(shared.call(&request(), &mut sink));
    assert_eq!(
        outcome.result().unwrap().text().next().unwrap().as_str(),
        "scripted"
    );
    assert_eq!(client.calls(), 1);
    assert_eq!(client.log()[0].state().kind(), StateKind::Succeeded);
    let folded = sink
        .fold(outcome.result().unwrap().model().cloned())
        .unwrap();
    assert_eq!(&folded, outcome.result().unwrap());
}

#[test]
fn given_capabilities_built_from_everything_when_changed_then_strictness_refuses_like_a_real_client()
 {
    let spec = CapabilitiesSpec {
        reasoning_levels: Accepted::only([ReasoningLevel::Low]),
        ..CapabilitiesSpec::everything()
    };
    let client = MockClient::new(settings(), Capabilities::new(spec).unwrap());
    let mut sink = RecordingSink::new();
    let outcome = block_on(client.call(&request(), &mut sink));
    assert!(matches!(
        outcome.result(),
        Err(ClientError::ReasoningNotSupported { .. })
    ));
    assert!(sink.is_empty());
}

#[test]
fn given_a_reply_function_and_a_helper_when_a_tool_call_is_scripted_then_ids_are_distinct_per_call()
{
    let client = MockClient::new(settings(), Capabilities::everything());
    client.set_fallback(ReplySource::function(|call| {
        let call = ToolCall {
            id: tool_call_id("call", call.sequence(), None),
            name: ToolName::new("search").unwrap(),
            arguments: serde_json::json!({ "q": "rust" }),
        };
        let step = Step::new(
            vec![AssistantBlock::ToolCall(call)],
            StopReason::AwaitingToolResults,
            None,
            None,
        )
        .unwrap();
        Reply::success(step).build()
    }));
    let mut ids = Vec::new();
    for _ in 0..2 {
        let mut sink = RecordingSink::new();
        let outcome = block_on(client.call(&request(), &mut sink));
        ids.push(
            outcome
                .result()
                .unwrap()
                .tool_calls()
                .next()
                .unwrap()
                .id
                .to_string(),
        );
    }
    assert_eq!(ids, ["call-0", "call-1"]);
}

#[test]
fn given_a_rule_and_a_gate_when_a_test_drives_a_call_by_hand_then_the_gate_decides_when_it_goes_on()
{
    let gate = Gate::new();
    let client = MockClient::new(settings(), Capabilities::everything());
    client.push_rule(Rule::always(
        Reply::success(answer("held"))
            .gate_at(1, &gate)
            .chunking(Chunking::none().text(2))
            .build()
            .unwrap(),
    ));
    let request_value = request();
    let mut sink = RecordingSink::new();
    let mut call = std::pin::pin!(client.call(&request_value, &mut sink));
    let mut context = Context::from_waker(Waker::noop());
    assert!(call.as_mut().poll(&mut context).is_pending());
    assert_eq!(gate.waiting(), 1);
    gate.open();
    assert!(call.as_mut().poll(&mut context).is_ready());
}

#[test]
fn given_a_step_when_events_are_derived_then_the_list_ends_with_the_finish_event() {
    let derived = derive_events(&answer("abc"), &Chunking::none()).unwrap();
    assert_eq!(derived.before_finish() + 1, derived.events().len());
}

#[test]
fn given_a_call_waiting_at_a_gate_when_another_thread_opens_it_then_the_parked_caller_goes_on() {
    let gate = Gate::new();
    let client = MockClient::new(settings(), Capabilities::everything());
    client.push_rule(Rule::always(
        Reply::success(answer("released"))
            .gate_at(0, &gate)
            .build()
            .unwrap(),
    ));
    let opener = {
        let gate = gate.clone();
        std::thread::spawn(move || {
            block_on(gate.wait_for_waiting(1));
            gate.open();
        })
    };
    let mut sink = RecordingSink::new();
    let outcome = block_on(client.call(&request(), &mut sink));
    opener.join().unwrap();
    assert!(outcome.result().is_ok());
    assert!(gate.is_open());
}
