use std::pin::pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::Poll;

use br_llm_clients_core::Client;

use crate::call_log::StateKind;
use crate::capabilities::Capabilities;
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

fn count(client: &MockClient, kind: StateKind) -> usize {
    kinds(client)
        .into_iter()
        .filter(|found| *found == kind)
        .count()
}

#[test]
fn given_a_state_change_by_release_or_open_when_they_return_then_states_and_number_are_already_updated()
 {
    let gate = Gate::new();
    let client = mock();
    client.set_fallback(gated(&[(0, &gate)]));
    let request_value = request();
    let mut sinks: Vec<RecordingSink> = (0..3).map(|_| RecordingSink::new()).collect();
    let mut calls: Vec<_> = sinks
        .iter_mut()
        .map(|sink| Box::pin(client.call(&request_value, sink)))
        .collect();
    for call in calls.iter_mut() {
        let _ = poll_once(&mut call.as_mut());
    }
    assert_eq!(count(&client, StateKind::WaitingAtGate), 3);
    gate.release(1);
    assert_eq!(gate.waiting(), 2);
    assert_eq!(count(&client, StateKind::WaitingAtGate), 2);
    assert_eq!(count(&client, StateKind::Running), 1);
    gate.open();
    assert_eq!(gate.waiting(), 0);
    assert_eq!(count(&client, StateKind::WaitingAtGate), 0);
    assert_eq!(count(&client, StateKind::Running), 3);
}

#[test]
fn given_a_wait_for_waiting_when_calls_arrive_then_the_waker_is_woken_and_the_wait_ends() {
    let gate = Gate::new();
    let client = mock();
    client.set_fallback(gated(&[(0, &gate)]));
    let wakes = Arc::new(WakeCount::default());
    let waker = wakes.waker();
    let mut wait = pin!(gate.wait_for_waiting(2));
    assert!(poll_with(&mut wait, &waker).is_pending());
    let request_value = request();
    let mut sink_a = RecordingSink::new();
    let mut sink_b = RecordingSink::new();
    let mut a = pin!(client.call(&request_value, &mut sink_a));
    let mut b = pin!(client.call(&request_value, &mut sink_b));
    let _ = poll_once(&mut a);
    assert!(poll_with(&mut wait, &waker).is_pending());
    let _ = poll_once(&mut b);
    assert!(wakes.count() >= 1);
    assert!(matches!(poll_with(&mut wait, &waker), Poll::Ready(())));
}

#[test]
fn given_a_wait_for_zero_calls_when_asked_then_it_is_ready_and_it_can_be_dropped_while_pending() {
    let gate = Gate::new();
    let mut zero = pin!(gate.wait_for_waiting(0));
    assert!(matches!(poll_once(&mut zero), Poll::Ready(())));
    let mut pending = Box::pin(gate.wait_for_waiting(3));
    assert!(poll_once(&mut pending.as_mut()).is_pending());
    drop(pending);
}

#[test]
fn given_one_gate_given_to_several_mocks_when_released_then_the_count_runs_across_mocks_by_waiting_time()
 {
    let gate = Gate::new();
    let first = mock();
    let second = MockClient::new(settings(), Capabilities::everything());
    first.set_fallback(gated(&[(0, &gate)]));
    second.set_fallback(gated(&[(0, &gate)]));
    let request_value = request();
    let mut sink_a = RecordingSink::new();
    let mut sink_b = RecordingSink::new();
    let mut sink_c = RecordingSink::new();
    let mut a = pin!(first.call(&request_value, &mut sink_a));
    let mut b = pin!(second.call(&request_value, &mut sink_b));
    let mut c = pin!(first.call(&request_value, &mut sink_c));
    let _ = poll_once(&mut a);
    let _ = poll_once(&mut b);
    let _ = poll_once(&mut c);
    assert_eq!(gate.waiting(), 3);
    gate.release(2);
    assert!(poll_once(&mut a).is_ready());
    assert!(poll_once(&mut b).is_ready());
    assert!(poll_once(&mut c).is_pending());
}

#[test]
fn given_a_clone_of_a_gate_when_opened_then_the_original_is_open_and_a_copied_reply_shares_the_gate()
 {
    let gate = Gate::new();
    let clone = gate.clone();
    let reply = gated(&[(0, &gate)]);
    let copied = reply.clone();
    clone.open();
    assert!(gate.is_open());
    let client = mock();
    client.push_reply(copied);
    let (outcome, _) = run(&client, &request());
    assert!(outcome.result().is_ok());
}

#[test]
fn given_arrivals_in_progress_when_the_number_is_read_before_the_log_then_the_log_is_never_older() {
    let gate = Gate::new();
    let client = mock();
    client.set_fallback(gated(&[(0, &gate)]));
    let stop = Arc::new(AtomicBool::new(false));
    let reader = {
        let (gate, client, stop) = (gate.clone(), client.clone(), Arc::clone(&stop));
        std::thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                let number = gate.waiting();
                assert!(count(&client, StateKind::WaitingAtGate) >= number);
            }
        })
    };
    let callers: Vec<_> = (0..16)
        .map(|_| {
            let client = client.clone();
            std::thread::spawn(move || run(&client, &request()))
        })
        .collect();
    block_on(gate.wait_for_waiting(16));
    stop.store(true, Ordering::SeqCst);
    reader.join().unwrap();
    gate.open();
    for caller in callers {
        assert!(caller.join().unwrap().0.result().is_ok());
    }
}

#[test]
fn given_releases_in_progress_when_the_number_is_read_before_the_log_then_the_log_is_never_older() {
    let gate = Gate::new();
    let client = mock();
    client.set_fallback(gated(&[(0, &gate)]));
    let callers: Vec<_> = (0..16)
        .map(|_| {
            let client = client.clone();
            std::thread::spawn(move || run(&client, &request()))
        })
        .collect();
    block_on(gate.wait_for_waiting(16));
    let stop = Arc::new(AtomicBool::new(false));
    let reader = {
        let (gate, client, stop) = (gate.clone(), client.clone(), Arc::clone(&stop));
        std::thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                let number = gate.waiting();
                assert!(count(&client, StateKind::WaitingAtGate) <= number);
            }
        })
    };
    for _ in 0..16 {
        gate.release(1);
    }
    stop.store(true, Ordering::SeqCst);
    reader.join().unwrap();
    for caller in callers {
        assert!(caller.join().unwrap().0.result().is_ok());
    }
}
