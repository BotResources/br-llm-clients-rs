use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use br_llm_clients_core::{Client, StreamSink};
use br_llm_messages::StreamEvent;

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
        Reply::success(text_step("ok"))
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
    client.set_fallback(Reply::success(text_step("ok")).build().unwrap());
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
    let (outcome, _) = run(&client, &request());
    assert!(outcome.result().is_ok());
}

#[test]
fn given_a_waker_that_panics_when_an_event_arrives_then_the_other_waiters_are_woken_and_the_sink_stays_usable()
 {
    let sink = RecordingSink::new();
    let survivor = Arc::new(WakeCount::default());
    let mut doomed = Box::pin(sink.wait_for_events(1));
    let mut waiting = Box::pin(sink.wait_for_events(1));
    assert!(poll_with(&mut doomed.as_mut(), &panicking_waker()).is_pending());
    assert!(poll_with(&mut waiting.as_mut(), &survivor.waker()).is_pending());
    let mut writer = sink.clone();
    let sent = catch_unwind(AssertUnwindSafe(|| {
        block_on(writer.send(StreamEvent::BlockEnd { index: 0 }))
    }));
    assert!(sent.is_err());
    assert_eq!(survivor.count(), 1);
    assert_eq!(sink.len(), 1);
    block_on(writer.send(StreamEvent::BlockEnd { index: 1 }));
    assert_eq!(sink.len(), 2);
}
