use std::pin::pin;
use std::sync::Arc;
use std::task::Poll;

use br_llm_clients_core::{Client, StreamSink};
use br_llm_messages::{DeltaOp, ResponseDelta};

use crate::recording_sink::RecordingSink;
use crate::reply::Reply;
use crate::test_support::*;

fn event(index: usize) -> ResponseDelta {
    delta(DeltaOp::text_appended(
        crate::fixtures::block_id(),
        index.to_string(),
    ))
}

fn markers(count: usize) -> Vec<ResponseDelta> {
    (0..count).map(event).collect()
}

#[test]
fn given_the_sink_when_compiled_then_it_is_usable_as_a_trait_object() {
    let boxed: Box<dyn StreamSink> = Box::new(RecordingSink::new());
    drop(boxed);
}

#[test]
fn given_events_when_sent_then_they_are_kept_in_the_order_received() {
    let mut sink = RecordingSink::new();
    let sent = markers(5);
    for item in &sent {
        block_on(sink.send(item.clone()));
    }
    assert_eq!(sink.deltas(), sent);
    assert_eq!(sink.len(), 5);
    assert!(!sink.is_empty());
}

#[test]
fn given_a_send_when_polled_once_then_it_is_ready_at_once_so_a_sink_is_always_ready() {
    let mut sink = RecordingSink::new();
    let mut future = pin!(sink.send(started_delta()));
    assert!(matches!(poll_once(&mut future), Poll::Ready(())));
}

#[test]
fn given_a_send_future_that_is_never_polled_when_dropped_then_nothing_is_recorded() {
    let mut sink = RecordingSink::new();
    drop(sink.send(started_delta()));
    assert!(sink.is_empty());
}

#[test]
fn given_a_clone_when_events_are_sent_through_one_handle_then_the_other_reads_them_at_once() {
    let reader = RecordingSink::new();
    let mut writer = reader.clone();
    let one = event(1);
    block_on(writer.send(one.clone()));
    assert_eq!(reader.deltas(), [one]);
}

#[test]
fn given_take_when_called_then_it_returns_the_events_and_empties_the_sink() {
    let mut sink = RecordingSink::new();
    let sent = markers(2);
    block_on(sink.send(sent[0].clone()));
    block_on(sink.send(sent[1].clone()));
    assert_eq!(sink.take(), sent);
    assert!(sink.is_empty());
    assert_eq!(sink.deltas(), []);
}

#[test]
fn given_a_sink_used_for_two_calls_when_read_then_it_holds_the_deltas_of_both_in_order() {
    let client = mock();
    client.push_reply(Reply::success(text_response("one")).build().unwrap());
    client.push_reply(Reply::success(text_response("two")).build().unwrap());
    let mut sink = RecordingSink::new();
    block_on(client.call(&request(), &mut sink));
    let single = sink.len();
    block_on(client.call(&request(), &mut sink));
    assert_eq!(single, 5);
    assert_eq!(sink.len(), single * 2);
}

#[test]
fn given_the_deltas_of_a_call_when_folded_then_the_response_comes_back() {
    let client = mock();
    client.push_reply(Reply::success(calls_response(1)).build().unwrap());
    let (outcome, sink) = run(&client, &request());
    assert_eq!(sink.fold().unwrap(), response_from(&outcome));
}

#[test]
fn given_deltas_that_do_not_fold_when_folded_then_an_error_with_a_reason() {
    let mut sink = RecordingSink::new();
    block_on(sink.send(event(0)));
    let error = sink.fold().unwrap_err();
    assert!(!error.reason().is_empty());
}

#[test]
fn given_a_sink_with_no_delta_when_folded_then_an_error() {
    assert!(RecordingSink::new().fold().is_err());
}

#[test]
fn given_deltas_that_stop_before_completed_when_folded_then_an_error() {
    let client = mock();
    client.push_reply(
        Reply::cut(
            text_response("hi"),
            3,
            br_llm_clients_core::ClientError::Transport,
        )
        .build()
        .unwrap(),
    );
    let (_, sink) = run(&client, &request());
    assert!(sink.fold().is_err());
}

#[test]
fn given_the_deltas_of_two_calls_when_folded_then_an_error() {
    let client = mock();
    client.push_reply(Reply::success(text_response("one")).build().unwrap());
    client.push_reply(Reply::success(text_response("two")).build().unwrap());
    let mut sink = RecordingSink::new();
    block_on(client.call(&request(), &mut sink));
    block_on(client.call(&request(), &mut sink));
    assert!(sink.fold().is_err());
}

#[test]
fn given_a_wait_for_deltas_when_deltas_arrive_then_the_waker_is_woken_and_the_wait_ends() {
    let sink = RecordingSink::new();
    let wakes = Arc::new(WakeCount::default());
    let waker = wakes.waker();
    let mut wait = pin!(sink.wait_for_deltas(2));
    assert!(poll_with(&mut wait, &waker).is_pending());
    let mut writer = sink.clone();
    block_on(writer.send(event(0)));
    assert!(poll_with(&mut wait, &waker).is_pending());
    block_on(writer.send(event(1)));
    assert!(wakes.count() >= 1);
    assert!(matches!(poll_with(&mut wait, &waker), Poll::Ready(())));
}

#[test]
fn given_a_wait_already_satisfied_or_dropped_while_pending_then_no_harm_is_done() {
    let sink = RecordingSink::new();
    let mut zero = pin!(sink.wait_for_deltas(0));
    assert!(matches!(poll_once(&mut zero), Poll::Ready(())));
    let mut pending = Box::pin(sink.wait_for_deltas(9));
    assert!(poll_once(&mut pending.as_mut()).is_pending());
    drop(pending);
    let mut writer = sink.clone();
    block_on(writer.send(event(0)));
    assert_eq!(sink.len(), 1);
}

#[test]
fn given_a_reader_on_another_thread_when_a_call_streams_then_the_reader_sees_deltas_while_the_call_runs()
 {
    let gate = crate::gate::Gate::new();
    let client = mock();
    client.push_reply(
        Reply::success(text_response("hello"))
            .gate_at(2, &gate)
            .build()
            .unwrap(),
    );
    let sink = RecordingSink::new();
    let caller = {
        let (client, sink) = (client.clone(), sink.clone());
        std::thread::spawn(move || {
            let mut handle = sink;
            block_on(client.call(&request(), &mut handle))
        })
    };
    block_on(sink.wait_for_deltas(2));
    assert_eq!(sink.len(), 2);
    gate.open();
    assert!(caller.join().unwrap().result().is_ok());
    assert!(sink.len() > 2);
}
