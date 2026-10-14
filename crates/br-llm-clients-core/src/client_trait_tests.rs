use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use br_llm_messages::{BlockKind, ModelId, StreamEvent, WireMessage, WireUserBlock};

use crate::test_support::{block_on, text};
use crate::{
    BoxFuture, CacheRequirement, CallOutcome, CallReport, Client, ClientError, ClientRequest,
    ClientSettings, MaxOutputTokens, OutputMode, ReasoningLevel, StreamSink,
};

struct YieldOnce(bool);

impl Future for YieldOnce {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
        if self.0 {
            return Poll::Ready(());
        }
        self.0 = true;
        context.waker().wake_by_ref();
        Poll::Pending
    }
}

#[derive(Default)]
struct Journal {
    entries: Vec<String>,
}

impl StreamSink for Journal {
    fn send<'a>(&'a mut self, event: StreamEvent) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            let StreamEvent::BlockStart { index, .. } = event else {
                return;
            };
            self.entries.push(format!("enter {index}"));
            YieldOnce(false).await;
            self.entries.push(format!("leave {index}"));
        })
    }
}

struct Streaming {
    blocks: usize,
    settings: ClientSettings,
}

impl Streaming {
    fn new(blocks: usize) -> Self {
        Self {
            blocks,
            settings: settings("model-x", ReasoningLevel::Off),
        }
    }
}

fn settings(model: &str, reasoning: ReasoningLevel) -> ClientSettings {
    ClientSettings::new(
        ModelId::new(model).unwrap(),
        reasoning,
        CacheRequirement::NotRequired,
        MaxOutputTokens::new(4096).unwrap(),
    )
}

impl Client for Streaming {
    fn call<'a>(
        &'a self,
        _request: &'a ClientRequest,
        sink: &'a mut dyn StreamSink,
    ) -> BoxFuture<'a, CallOutcome> {
        Box::pin(async move {
            for index in 0..self.blocks {
                sink.send(StreamEvent::BlockStart {
                    index,
                    kind: BlockKind::Text,
                })
                .await;
            }
            CallOutcome::failure(
                CallReport::new(Duration::from_millis(1)),
                ClientError::Transport,
            )
        })
    }

    fn settings(&self) -> &ClientSettings {
        &self.settings
    }
}

fn request() -> ClientRequest {
    let message = WireMessage::User {
        content: vec![WireUserBlock::Text { text: text("hi") }],
    };
    ClientRequest::new(None, vec![message], vec![], OutputMode::Plain).unwrap()
}

fn assert_send<T: Send>() {}

fn assert_send_sync<T: Send + Sync + ?Sized>() {}

fn assert_future_is_send<'a>(_: &BoxFuture<'a, CallOutcome>) {
    assert_send::<BoxFuture<'a, CallOutcome>>();
}

#[test]
fn given_the_traits_when_compiled_then_clients_are_shareable_and_sinks_movable_as_trait_objects() {
    assert_send_sync::<dyn Client>();
    assert_send_sync::<Box<dyn Client>>();
    assert_send_sync::<Arc<dyn Client>>();
    assert_send::<Box<dyn StreamSink>>();
    assert_send::<BoxFuture<'static, ()>>();
    assert_send::<BoxFuture<'static, CallOutcome>>();
}

#[test]
fn given_a_client_held_as_a_trait_object_when_asked_then_it_gives_back_the_settings_it_is_bound_to()
{
    let bound = settings("model-y", ReasoningLevel::High);
    let client: Box<dyn Client> = Box::new(Streaming {
        blocks: 0,
        settings: bound.clone(),
    });
    let read: fn(&dyn Client) -> &ClientSettings = |client| client.settings();
    assert_eq!(read(client.as_ref()), &bound);
}

#[test]
fn given_a_call_running_when_the_settings_are_asked_then_they_are_given_without_waiting() {
    let client: Arc<dyn Client> = Arc::new(Streaming::new(2));
    let request = request();
    let mut sink = Journal::default();
    let call = client.call(&request, &mut sink);
    assert_eq!(client.settings(), &settings("model-x", ReasoningLevel::Off));
    block_on(call);
}

#[test]
fn given_a_call_through_a_trait_object_when_made_then_its_future_is_send() {
    let client: Box<dyn Client> = Box::new(Streaming::new(0));
    let request = request();
    let mut sink = Journal::default();
    let future = client.call(&request, &mut sink);
    assert_future_is_send(&future);
    block_on(future);
}

#[test]
fn given_a_call_when_it_returns_then_the_request_and_the_sink_are_still_usable_by_the_caller() {
    let client: Box<dyn Client> = Box::new(Streaming::new(2));
    let request = request();
    let before = request.clone();
    let mut sink = Journal::default();
    block_on(client.call(&request, &mut sink));
    assert_eq!(request, before);
    assert_eq!(sink.entries.len(), 4);
    block_on(sink.send(StreamEvent::BlockStart {
        index: 9,
        kind: BlockKind::Text,
    }));
    assert_eq!(sink.entries.last().map(String::as_str), Some("leave 9"));
}

#[test]
fn given_a_sink_that_waits_when_a_call_streams_then_each_event_is_handled_before_the_next_is_sent()
{
    let client: Box<dyn Client> = Box::new(Streaming::new(3));
    let request = request();
    let mut sink = Journal::default();
    block_on(client.call(&request, &mut sink));
    assert_eq!(
        sink.entries,
        [
            "enter 0", "leave 0", "enter 1", "leave 1", "enter 2", "leave 2"
        ]
    );
}

#[test]
fn given_one_shared_client_when_called_from_several_threads_then_each_call_owns_its_sink() {
    let client: Arc<dyn Client> = Arc::new(Streaming::new(2));
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let client = Arc::clone(&client);
            std::thread::spawn(move || {
                let request = request();
                let mut sink = Journal::default();
                block_on(client.call(&request, &mut sink));
                sink.entries
            })
        })
        .collect();
    for handle in handles {
        assert_eq!(
            handle.join().unwrap(),
            ["enter 0", "leave 0", "enter 1", "leave 1"]
        );
    }
}
