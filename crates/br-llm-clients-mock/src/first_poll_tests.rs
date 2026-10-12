use std::pin::pin;
use std::task::{Context, Poll};

use br_llm_clients_core::{BoxFuture, Client, ClientError, StreamSink};
use br_llm_messages::StreamEvent;

use crate::gate::Gate;
use crate::mock_client::MockClient;
use crate::recording_sink::RecordingSink;
use crate::reply::Reply;
use crate::test_support::*;

fn says(value: &str) -> Reply {
    Reply::success(text_step(value)).build().unwrap()
}

#[test]
fn given_a_call_that_meets_no_gate_no_hang_and_a_ready_sink_when_polled_once_with_a_no_op_waker_then_it_is_complete()
 {
    let client = mock();
    client.push_reply(says("one poll"));
    client.push_reply(Reply::failure(ClientError::Transport).build().unwrap());
    client.push_reply(
        Reply::cut(text_step("x"), 2, ClientError::StreamTruncated)
            .build()
            .unwrap(),
    );
    client.push_reply(
        Reply::broken_stream(vec![], ClientError::Transport)
            .build()
            .unwrap(),
    );
    client.push_reply(calls_step_reply());
    let request_value = request();
    for _ in 0..5 {
        let mut sink = RecordingSink::new();
        let mut call = pin!(client.call(&request_value, &mut sink));
        assert!(matches!(poll_once(&mut call), Poll::Ready(_)));
    }
    let mut sink = RecordingSink::new();
    let mut exhausted = pin!(client.call(&request_value, &mut sink));
    assert!(matches!(poll_once(&mut exhausted), Poll::Ready(_)));
}

fn calls_step_reply() -> Reply {
    Reply::success(calls_step(&["a", "b"]))
        .chunking(crate::Chunking::none().tool_arguments(1).text(1))
        .build()
        .unwrap()
}

#[test]
fn given_a_refused_call_when_polled_once_with_a_no_op_waker_then_it_is_complete() {
    let client = MockClient::new(settings(), everything_caps());
    let bad = request_of(
        vec![assistant_text("x")],
        vec![],
        br_llm_clients_core::OutputMode::Plain,
    );
    let mut sink = RecordingSink::new();
    let mut call = pin!(client.call(&bad, &mut sink));
    assert!(matches!(poll_once(&mut call), Poll::Ready(_)));
}

#[test]
fn given_an_open_gate_at_every_position_when_polled_once_then_it_is_complete() {
    let gate = Gate::new();
    gate.open();
    let reply = Reply::success(text_step("x"))
        .gate_at(0, &gate)
        .gate_at(1, &gate)
        .gate_at(2, &gate)
        .gate_at(3, &gate)
        .build()
        .unwrap();
    let client = mock();
    client.push_reply(reply);
    let mut sink = RecordingSink::new();
    let request_value = request();
    let mut call = pin!(client.call(&request_value, &mut sink));
    assert!(matches!(poll_once(&mut call), Poll::Ready(_)));
}

struct YieldingSink {
    events: usize,
    yielded: bool,
}

struct YieldOnce(bool);

impl std::future::Future for YieldOnce {
    type Output = ();

    fn poll(mut self: std::pin::Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
        if self.0 {
            return Poll::Ready(());
        }
        self.0 = true;
        context.waker().wake_by_ref();
        Poll::Pending
    }
}

impl StreamSink for YieldingSink {
    fn send<'a>(&'a mut self, _event: StreamEvent) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            self.yielded = true;
            YieldOnce(false).await;
            self.events += 1;
        })
    }
}

#[test]
fn given_a_sink_that_is_not_ready_when_a_call_streams_then_the_call_waits_for_the_sink_and_for_nothing_else()
 {
    let client = mock();
    client.push_reply(says("slow sink"));
    let mut sink = YieldingSink {
        events: 0,
        yielded: false,
    };
    let request_value = request();
    let mut polls = 0;
    let outcome = {
        let mut call = pin!(client.call(&request_value, &mut sink));
        loop {
            polls += 1;
            if let Poll::Ready(outcome) = poll_once(&mut call) {
                break outcome;
            }
        }
    };
    assert!(sink.yielded);
    assert_eq!(sink.events, 4);
    assert_eq!(polls, 5);
    assert!(outcome.result().is_ok());
}
