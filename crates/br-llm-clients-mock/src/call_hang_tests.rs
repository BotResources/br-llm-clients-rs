use std::pin::pin;
use std::sync::Arc;
use std::task::Poll;

use br_llm_clients_core::{Client, ClientError};
use br_llm_messages::DeltaOp;

use crate::call_log::StateKind;
use crate::chunking::Chunking;
use crate::gate::Gate;
use crate::recording_sink::RecordingSink;
use crate::reply::Reply;
use crate::reply_source::ReplySource;
use crate::test_support::*;

#[test]
fn given_a_hang_at_the_full_count_when_called_then_every_delta_but_the_completed_is_sent_and_the_call_never_ends()
 {
    let response = text_response("hello");
    let request_value = request();
    let expected = expected_deltas(&request_value, &response, &Chunking::none());
    let full = expected.len() - 1;
    let client = mock();
    client.push_reply(Reply::hang(response, full).build().unwrap());
    let sink = RecordingSink::new();
    let mut handle = sink.clone();
    let mut call = pin!(client.call(&request_value, &mut handle));
    assert!(poll_once(&mut call).is_pending());
    assert_eq!(sink.deltas(), expected[..full]);
    assert!(
        !sink
            .deltas()
            .iter()
            .any(|delta| matches!(delta.op(), DeltaOp::ResponseCompleted { .. }))
    );
    assert_eq!(client.log()[0].state().kind(), StateKind::Hanging);
}

#[test]
fn given_a_hang_at_zero_when_called_then_no_delta_is_sent_and_the_call_hangs() {
    let client = mock();
    client.push_reply(Reply::hang(text_response("x"), 0).build().unwrap());
    let request_value = request();
    let sink = RecordingSink::new();
    let mut handle = sink.clone();
    let mut call = pin!(client.call(&request_value, &mut handle));
    assert!(poll_once(&mut call).is_pending());
    assert!(sink.is_empty());
}

#[test]
fn given_a_gate_at_the_hang_position_when_called_then_the_call_waits_at_the_gate_before_it_hangs() {
    let gate = Gate::new();
    let client = mock();
    client.push_reply(
        Reply::hang(text_response("x"), 1)
            .gate_at(1, &gate)
            .build()
            .unwrap(),
    );
    let request_value = request();
    let mut sink = RecordingSink::new();
    let mut call = pin!(client.call(&request_value, &mut sink));
    assert!(poll_once(&mut call).is_pending());
    assert_eq!(client.log()[0].state().kind(), StateKind::WaitingAtGate);
    gate.open();
    assert!(poll_once(&mut call).is_pending());
    assert_eq!(client.log()[0].state().kind(), StateKind::Hanging);
}

#[test]
fn given_a_gate_at_the_cut_position_when_called_then_the_call_waits_before_it_gives_the_error() {
    let gate = Gate::new();
    let client = mock();
    client.push_reply(
        Reply::cut(text_response("x"), 2, ClientError::StreamTruncated)
            .gate_at(2, &gate)
            .build()
            .unwrap(),
    );
    let request_value = request();
    let mut sink = RecordingSink::new();
    let mut call = pin!(client.call(&request_value, &mut sink));
    assert!(poll_once(&mut call).is_pending());
    gate.open();
    let Poll::Ready(outcome) = poll_once(&mut call) else {
        panic!("the call should end");
    };
    assert_eq!(error_of(&outcome), ClientError::StreamTruncated);
}

#[test]
fn given_a_hanging_call_when_the_task_is_woken_by_others_then_it_stays_pending_and_is_never_woken_by_the_mock()
 {
    let client = mock();
    client.push_reply(Reply::hang(text_response("x"), 1).build().unwrap());
    client.set_fallback(Reply::success(text_response("y")).build().unwrap());
    let wakes = Arc::new(WakeCount::default());
    let waker = wakes.waker();
    let request_value = request();
    let mut sink = RecordingSink::new();
    let mut hung = pin!(client.call(&request_value, &mut sink));
    assert!(poll_with(&mut hung, &waker).is_pending());
    run(&client, &request());
    assert_eq!(wakes.count(), 0);
}

#[test]
fn given_a_call_waiting_at_a_gate_when_the_gate_opens_then_the_waker_of_the_call_is_woken() {
    let gate = Gate::new();
    let client = mock();
    client.push_reply(
        Reply::success(text_response("x"))
            .gate_at(0, &gate)
            .build()
            .unwrap(),
    );
    let wakes = Arc::new(WakeCount::default());
    let waker = wakes.waker();
    let request_value = request();
    let mut sink = RecordingSink::new();
    let mut call = pin!(client.call(&request_value, &mut sink));
    assert!(poll_with(&mut call, &waker).is_pending());
    assert_eq!(wakes.count(), 0);
    gate.open();
    assert_eq!(wakes.count(), 1);
    assert!(matches!(poll_with(&mut call, &waker), Poll::Ready(_)));
}

#[test]
fn given_a_released_call_when_release_returns_then_only_the_released_call_was_woken() {
    let gate = Gate::new();
    let client = mock();
    client.set_fallback(
        Reply::success(text_response("x"))
            .gate_at(0, &gate)
            .build()
            .unwrap(),
    );
    let first_wakes = Arc::new(WakeCount::default());
    let second_wakes = Arc::new(WakeCount::default());
    let request_value = request();
    let mut sink_a = RecordingSink::new();
    let mut sink_b = RecordingSink::new();
    let mut a = pin!(client.call(&request_value, &mut sink_a));
    let mut b = pin!(client.call(&request_value, &mut sink_b));
    assert!(poll_with(&mut a, &first_wakes.waker()).is_pending());
    assert!(poll_with(&mut b, &second_wakes.waker()).is_pending());
    gate.release(1);
    assert_eq!((first_wakes.count(), second_wakes.count()), (1, 0));
}

#[test]
fn given_script_entries_that_are_functions_when_claimed_then_the_prior_claims_are_the_entry_positions()
 {
    let client = mock();
    for _ in 0..3 {
        client.push_reply(ReplySource::function(|call| {
            Reply::text(format!("entry {}", call.claimed_before())).build()
        }));
    }
    let answers: Vec<String> = (0..3)
        .map(|_| text_of(&run(&client, &request()).0))
        .collect();
    assert_eq!(answers, ["entry 0", "entry 1", "entry 2"]);
}
