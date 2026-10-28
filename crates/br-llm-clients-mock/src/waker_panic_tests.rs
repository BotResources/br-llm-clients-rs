use std::future::Future;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use br_llm_clients_core::{BoxFuture, Client, StreamSink};
use br_llm_messages::ResponseDelta;

use crate::call_log::{CallState, StateKind};
use crate::gate::Gate;
use crate::recording_sink::RecordingSink;
use crate::reply::Reply;
use crate::test_support::*;
use crate::waker_support::panicking_waker;

#[test]
fn given_a_waker_that_panics_when_a_gate_opens_then_the_other_waiting_calls_are_still_woken() {
    let gate = Gate::new();
    let client = mock();
    client.set_fallback(
        Reply::success(text_response("ok"))
            .gate_at(0, &gate)
            .build()
            .unwrap(),
    );
    let request_value = request();
    let survivor = Arc::new(WakeCount::default());
    let mut sink_a = RecordingSink::new();
    let mut sink_b = RecordingSink::new();
    let mut doomed = Box::pin(client.call(&request_value, &mut sink_a));
    let mut waiting = Box::pin(client.call(&request_value, &mut sink_b));
    assert!(poll_with(&mut doomed.as_mut(), &panicking_waker()).is_pending());
    assert!(poll_with(&mut waiting.as_mut(), &survivor.waker()).is_pending());
    assert!(catch_unwind(AssertUnwindSafe(|| gate.open())).is_err());
    assert_eq!(survivor.count(), 1);
    assert!(gate.is_open());
    assert_eq!(gate.waiting(), 0);
    assert!(poll_with(&mut waiting.as_mut(), &survivor.waker()).is_ready());
}

#[test]
fn given_a_waker_that_panics_when_calls_begin_then_the_other_waiters_are_woken_and_the_mock_stays_usable()
 {
    let client = mock();
    client.set_fallback(Reply::success(text_response("ok")).build().unwrap());
    let survivor = Arc::new(WakeCount::default());
    let mut doomed = Box::pin(client.wait_for_calls(1));
    let mut waiting = Box::pin(client.wait_for_calls(1));
    assert!(poll_with(&mut doomed.as_mut(), &panicking_waker()).is_pending());
    assert!(poll_with(&mut waiting.as_mut(), &survivor.waker()).is_pending());
    let request_value = request();
    let mut sink = RecordingSink::new();
    let mut call = Box::pin(client.call(&request_value, &mut sink));
    let first = catch_unwind(AssertUnwindSafe(|| poll_once(&mut call.as_mut())));
    assert!(first.is_err());
    assert_eq!(survivor.count(), 1);
    assert_eq!(client.calls(), 1);
    drop(call);
    assert_eq!(client.log()[0].state(), &CallState::Dropped);
    let mut phantom = Box::pin(client.wait_for_state(StateKind::Running, 1).unwrap());
    assert!(poll_once(&mut phantom.as_mut()).is_pending());
    let (outcome, _) = run(&client, &request());
    assert!(outcome.result().is_ok());
}

#[test]
fn given_a_waker_that_panics_when_an_event_arrives_then_the_other_waiters_are_woken_and_the_sink_stays_usable()
 {
    let sink = RecordingSink::new();
    let survivor = Arc::new(WakeCount::default());
    let mut doomed = Box::pin(sink.wait_for_deltas(1));
    let mut waiting = Box::pin(sink.wait_for_deltas(1));
    assert!(poll_with(&mut doomed.as_mut(), &panicking_waker()).is_pending());
    assert!(poll_with(&mut waiting.as_mut(), &survivor.waker()).is_pending());
    let mut writer = sink.clone();
    let sent = catch_unwind(AssertUnwindSafe(|| block_on(writer.send(started_delta()))));
    assert!(sent.is_err());
    assert_eq!(survivor.count(), 1);
    assert_eq!(sink.len(), 1);
    block_on(writer.send(started_delta()));
    assert_eq!(sink.len(), 2);
}

struct PendingThenPanic {
    polled: bool,
}

impl Future for PendingThenPanic {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<()> {
        if self.polled {
            panic!("the sink failed");
        }
        self.polled = true;
        Poll::Pending
    }
}

struct FailingSink;

impl StreamSink for FailingSink {
    fn send<'a>(&'a mut self, _delta: ResponseDelta) -> BoxFuture<'a, ()> {
        Box::pin(PendingThenPanic { polled: false })
    }
}

#[test]
fn given_a_sink_that_panics_while_a_waiter_waker_panics_then_the_unwinding_does_not_abort_and_the_other_waiter_is_woken()
 {
    let client = mock();
    client.set_fallback(Reply::success(text_response("ok")).build().unwrap());
    let request_value = request();
    let mut sink = FailingSink;
    let mut call = Box::pin(client.call(&request_value, &mut sink));
    assert!(poll_once(&mut call.as_mut()).is_pending());
    let survivor = Arc::new(WakeCount::default());
    let mut doomed = Box::pin(client.wait_for_state(StateKind::Dropped, 1).unwrap());
    let mut waiting = Box::pin(client.wait_for_state(StateKind::Dropped, 1).unwrap());
    assert!(poll_with(&mut doomed.as_mut(), &panicking_waker()).is_pending());
    assert!(poll_with(&mut waiting.as_mut(), &survivor.waker()).is_pending());
    let polled = catch_unwind(AssertUnwindSafe(|| poll_once(&mut call.as_mut())));
    assert!(polled.is_err());
    assert_eq!(survivor.count(), 1);
    assert_eq!(client.log()[0].state(), &CallState::Dropped);
}
