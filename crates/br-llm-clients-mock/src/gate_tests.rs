use std::pin::pin;
use std::task::Poll;

use br_llm_clients_core::Client;

use crate::call_log::StateKind;
use crate::gate::Gate;
use crate::mock_client::MockClient;
use crate::recording_sink::RecordingSink;
use crate::reply::Reply;
use crate::test_support::*;

fn gated(gates: &[(usize, &Gate)]) -> Reply {
    let mut builder = Reply::success(text_step("gated"));
    for (position, gate) in gates {
        builder = builder.gate_at(*position, gate);
    }
    builder.build().unwrap()
}

fn kinds(client: &MockClient) -> Vec<StateKind> {
    client
        .log()
        .iter()
        .map(|call| call.state().kind())
        .collect()
}

#[test]
fn given_a_closed_gate_at_position_zero_when_called_then_no_event_is_sent_until_it_opens() {
    let gate = Gate::new();
    let client = mock();
    client.push_reply(gated(&[(0, &gate)]));
    let request_value = request();
    let sink = RecordingSink::new();
    let mut handle = sink.clone();
    let mut call = pin!(client.call(&request_value, &mut handle));
    assert!(poll_once(&mut call).is_pending());
    assert_eq!(sink.len(), 0);
    assert_eq!(gate.waiting(), 1);
    gate.open();
    assert!(matches!(poll_once(&mut call), Poll::Ready(_)));
    assert!(sink.len() > 1);
}

#[test]
fn given_a_gate_at_a_later_position_when_called_then_exactly_that_many_events_are_sent_first() {
    let gate = Gate::new();
    let client = mock();
    client.push_reply(gated(&[(2, &gate)]));
    let request_value = request();
    let sink = RecordingSink::new();
    let mut handle = sink.clone();
    let mut call = pin!(client.call(&request_value, &mut handle));
    assert!(poll_once(&mut call).is_pending());
    assert_eq!(sink.len(), 2);
}

#[test]
fn given_a_gate_at_the_last_position_of_a_success_when_called_then_the_call_waits_before_the_finish_event()
 {
    let gate = Gate::new();
    let client = mock();
    let count_before_finish = 3;
    client.push_reply(gated(&[(count_before_finish, &gate)]));
    let request_value = request();
    let sink = RecordingSink::new();
    let mut handle = sink.clone();
    let mut call = pin!(client.call(&request_value, &mut handle));
    assert!(poll_once(&mut call).is_pending());
    assert_eq!(sink.len(), count_before_finish);
    gate.open();
    assert!(matches!(poll_once(&mut call), Poll::Ready(_)));
    assert_eq!(sink.len(), count_before_finish + 1);
}

#[test]
fn given_an_open_gate_when_a_call_reaches_it_then_it_is_not_a_wait_and_the_call_ends_at_its_first_poll()
 {
    let gate = Gate::new();
    gate.open();
    let client = mock();
    client.push_reply(gated(&[(0, &gate), (1, &gate), (2, &gate)]));
    let request_value = request();
    let mut sink = RecordingSink::new();
    let mut call = pin!(client.call(&request_value, &mut sink));
    assert!(matches!(poll_once(&mut call), Poll::Ready(_)));
    assert_eq!(gate.waiting(), 0);
}

#[test]
fn given_an_opened_gate_when_calls_waited_and_arrive_later_then_all_pass_and_the_opening_is_final()
{
    let gate = Gate::new();
    let client = mock();
    client.set_fallback(gated(&[(0, &gate)]));
    let request_value = request();
    let mut early_sink = RecordingSink::new();
    let mut early = pin!(client.call(&request_value, &mut early_sink));
    assert!(poll_once(&mut early).is_pending());
    gate.open();
    assert!(gate.is_open());
    assert_eq!(gate.waiting(), 0);
    assert!(matches!(poll_once(&mut early), Poll::Ready(_)));
    let mut late_sink = RecordingSink::new();
    let mut late = pin!(client.call(&request_value, &mut late_sink));
    assert!(matches!(poll_once(&mut late), Poll::Ready(_)));
    gate.open();
    assert!(gate.is_open());
}

#[test]
fn given_waiting_calls_when_released_by_number_then_the_longest_waiting_go_first_and_the_gate_stays_closed()
 {
    let gate = Gate::new();
    let client = mock();
    client.set_fallback(gated(&[(0, &gate)]));
    let request_value = request();
    let mut sinks: Vec<RecordingSink> = (0..4).map(|_| RecordingSink::new()).collect();
    let mut calls: Vec<_> = sinks
        .iter_mut()
        .map(|sink| Box::pin(client.call(&request_value, sink)))
        .collect();
    for call in calls.iter_mut() {
        assert!(poll_once(&mut call.as_mut()).is_pending());
    }
    assert_eq!(gate.waiting(), 4);
    gate.release(2);
    assert_eq!(gate.waiting(), 2);
    assert!(!gate.is_open());
    let ready: Vec<bool> = calls
        .iter_mut()
        .map(|call| poll_once(&mut call.as_mut()).is_ready())
        .collect();
    assert_eq!(ready, [true, true, false, false]);
    assert_eq!(
        kinds(&client),
        [
            StateKind::Succeeded,
            StateKind::Succeeded,
            StateKind::WaitingAtGate,
            StateKind::WaitingAtGate
        ]
    );
}

#[test]
fn given_fewer_waiting_calls_than_released_when_released_then_only_those_go_and_later_calls_still_wait()
 {
    let gate = Gate::new();
    let client = mock();
    client.set_fallback(gated(&[(0, &gate)]));
    let request_value = request();
    let mut first_sink = RecordingSink::new();
    let mut first = pin!(client.call(&request_value, &mut first_sink));
    assert!(poll_once(&mut first).is_pending());
    gate.release(5);
    assert!(matches!(poll_once(&mut first), Poll::Ready(_)));
    let mut later_sink = RecordingSink::new();
    let mut later = pin!(client.call(&request_value, &mut later_sink));
    assert!(poll_once(&mut later).is_pending());
    assert_eq!(gate.waiting(), 1);
}

#[test]
fn given_a_release_when_the_call_reaches_a_later_position_of_the_same_gate_then_it_waits_again() {
    let gate = Gate::new();
    let client = mock();
    client.push_reply(gated(&[(0, &gate), (2, &gate)]));
    let request_value = request();
    let sink = RecordingSink::new();
    let mut handle = sink.clone();
    let mut call = pin!(client.call(&request_value, &mut handle));
    assert!(poll_once(&mut call).is_pending());
    gate.release(1);
    assert!(poll_once(&mut call).is_pending());
    assert_eq!(sink.len(), 2);
    assert_eq!(gate.waiting(), 1);
    gate.release(1);
    assert!(matches!(poll_once(&mut call), Poll::Ready(_)));
}

#[test]
fn given_a_release_when_the_call_reaches_another_gate_then_it_waits_there_in_the_order_given() {
    let first_gate = Gate::new();
    let second_gate = Gate::new();
    let client = mock();
    client.push_reply(gated(&[(1, &first_gate), (1, &second_gate)]));
    let request_value = request();
    let mut sink = RecordingSink::new();
    let mut call = pin!(client.call(&request_value, &mut sink));
    assert!(poll_once(&mut call).is_pending());
    assert_eq!((first_gate.waiting(), second_gate.waiting()), (1, 0));
    first_gate.release(1);
    assert!(poll_once(&mut call).is_pending());
    assert_eq!((first_gate.waiting(), second_gate.waiting()), (0, 1));
    second_gate.open();
    assert!(matches!(poll_once(&mut call), Poll::Ready(_)));
}

#[test]
fn given_a_dropped_waiting_call_when_the_gate_is_read_then_the_number_drops_and_the_log_says_dropped()
 {
    let gate = Gate::new();
    let client = mock();
    client.set_fallback(gated(&[(0, &gate)]));
    let request_value = request();
    let mut sink = RecordingSink::new();
    let mut call = Box::pin(client.call(&request_value, &mut sink));
    assert!(poll_once(&mut call.as_mut()).is_pending());
    assert_eq!(gate.waiting(), 1);
    drop(call);
    assert_eq!(gate.waiting(), 0);
    assert_eq!(kinds(&client), [StateKind::Dropped]);
}

#[test]
fn given_a_released_call_that_is_dropped_before_it_resumes_then_nothing_leaks_into_later_releases()
{
    let gate = Gate::new();
    let client = mock();
    client.set_fallback(gated(&[(0, &gate)]));
    let request_value = request();
    let mut dropped_sink = RecordingSink::new();
    let mut dropped = Box::pin(client.call(&request_value, &mut dropped_sink));
    assert!(poll_once(&mut dropped.as_mut()).is_pending());
    gate.release(1);
    drop(dropped);
    let mut next_sink = RecordingSink::new();
    let mut next = pin!(client.call(&request_value, &mut next_sink));
    assert!(poll_once(&mut next).is_pending());
    assert_eq!(gate.waiting(), 1);
}
