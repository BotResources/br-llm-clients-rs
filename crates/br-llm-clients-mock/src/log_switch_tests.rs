use std::pin::pin;
use std::sync::Arc;
use std::task::Poll;

use br_llm_clients_core::{Client, ClientError, OutputMode};

use crate::call_log::StateKind;
use crate::capabilities::Capabilities;
use crate::gate::Gate;
use crate::log_disabled::LogDisabled;
use crate::mock_client::MockClient;
use crate::recording_sink::RecordingSink;
use crate::reply::Reply;
use crate::test_support::*;

fn says(value: &str) -> Reply {
    Reply::success(text_step(value)).build().unwrap()
}

fn gated(gate: &Gate) -> Reply {
    Reply::success(text_step("gated"))
        .gate_at(0, gate)
        .build()
        .unwrap()
}

fn unlogged() -> MockClient {
    MockClient::builder(settings(), Capabilities::everything())
        .log(false)
        .build()
}

fn kinds(client: &MockClient) -> Vec<StateKind> {
    client
        .log()
        .iter()
        .map(|call| call.state().kind())
        .collect()
}

#[test]
fn given_the_log_switched_off_when_calls_are_made_then_the_log_is_empty_and_counts_stay_exact() {
    let client = unlogged();
    client.push_reply(says("a"));
    client.push_reply(says("b"));
    client.push_reply(says("c"));
    run(&client, &request());
    run(&client, &request());
    assert!(client.log().is_empty());
    assert_eq!(client.calls(), 2);
    assert_eq!(client.unused_entries(), 1);
}

#[test]
fn given_the_log_switched_off_when_a_wait_on_state_is_asked_then_log_disabled_comes_at_once() {
    let client = unlogged();
    assert_eq!(
        client.wait_for_state(StateKind::Running, 1).err(),
        Some(LogDisabled)
    );
}

#[test]
fn given_the_log_switched_off_when_the_behaviour_is_compared_then_checks_script_and_rules_work_the_same()
 {
    let client = unlogged();
    client.push_reply(says("scripted"));
    let bad = request_of(vec![assistant_text("x")], vec![], OutputMode::Plain);
    let (refused, _) = run(&client, &bad);
    assert!(matches!(
        error_of(&refused),
        ClientError::InvalidMessages { .. }
    ));
    let (served, _) = run(&client, &request());
    assert_eq!(step_of(&served).text().next().unwrap().as_str(), "scripted");
    let (exhausted, _) = run(&client, &request());
    assert!(matches!(
        error_of(&exhausted),
        ClientError::ProviderRejected { .. }
    ));
}

#[test]
fn given_the_log_switched_off_when_waiting_for_calls_then_the_wait_still_works() {
    let client = unlogged();
    client.push_reply(says("x"));
    let mut wait = pin!(client.wait_for_calls(1));
    assert!(poll_once(&mut wait).is_pending());
    run(&client, &request());
    assert!(matches!(poll_once(&mut wait), Poll::Ready(())));
}

#[test]
fn given_a_wait_for_calls_when_calls_begin_then_the_waker_of_the_waiting_task_is_woken() {
    let client = mock();
    client.set_fallback(says("x"));
    let wakes = Arc::new(WakeCount::default());
    let waker = wakes.waker();
    let mut wait = pin!(client.wait_for_calls(2));
    assert!(poll_with(&mut wait, &waker).is_pending());
    run(&client, &request());
    assert!(poll_with(&mut wait, &waker).is_pending());
    assert!(wakes.count() >= 1);
    run(&client, &request());
    assert!(matches!(poll_with(&mut wait, &waker), Poll::Ready(())));
}

#[test]
fn given_a_wait_for_calls_that_is_already_satisfied_when_asked_then_it_is_ready_at_once() {
    let client = mock();
    run(&client, &request());
    let mut wait = pin!(client.wait_for_calls(1));
    assert!(matches!(poll_once(&mut wait), Poll::Ready(())));
    let mut zero = pin!(client.wait_for_calls(0));
    assert!(matches!(poll_once(&mut zero), Poll::Ready(())));
}

#[test]
fn given_a_wait_for_a_state_when_calls_enter_it_then_it_ends_and_it_looks_at_now_not_at_history() {
    let gate = Gate::new();
    let client = mock();
    client.set_fallback(gated(&gate));
    let request_value = request();
    let mut sinks = [RecordingSink::new(), RecordingSink::new()];
    let [sink_a, sink_b] = &mut sinks;
    let mut first = pin!(client.call(&request_value, sink_a));
    let mut second = pin!(client.call(&request_value, sink_b));
    let mut wait = pin!(client.wait_for_state(StateKind::WaitingAtGate, 2).unwrap());
    assert!(poll_once(&mut wait).is_pending());
    assert!(poll_once(&mut first).is_pending());
    assert!(poll_once(&mut wait).is_pending());
    assert!(poll_once(&mut second).is_pending());
    assert!(matches!(poll_once(&mut wait), Poll::Ready(())));
    gate.open();
    block_on(first.as_mut());
    block_on(second.as_mut());
    let mut past = pin!(client.wait_for_state(StateKind::WaitingAtGate, 1).unwrap());
    assert!(poll_once(&mut past).is_pending());
    let mut finished = pin!(client.wait_for_state(StateKind::Succeeded, 2).unwrap());
    assert!(matches!(poll_once(&mut finished), Poll::Ready(())));
}

#[test]
fn given_a_wait_for_an_error_state_when_calls_fail_then_whatever_the_error_they_count() {
    let client = mock();
    client.push_reply(Reply::failure(ClientError::Transport).build().unwrap());
    client.push_reply(Reply::failure(ClientError::QuotaExceeded).build().unwrap());
    run(&client, &request());
    run(&client, &request());
    let mut wait = pin!(client.wait_for_state(StateKind::Failed, 2).unwrap());
    assert!(matches!(poll_once(&mut wait), Poll::Ready(())));
}

#[test]
fn given_a_wait_that_is_dropped_when_the_log_changes_then_nothing_breaks() {
    let client = mock();
    client.set_fallback(says("x"));
    let wait = client.wait_for_calls(5);
    let mut pinned = Box::pin(wait);
    assert!(poll_once(&mut pinned.as_mut()).is_pending());
    drop(pinned);
    run(&client, &request());
    assert_eq!(client.calls(), 1);
}

#[test]
fn given_a_call_waiting_at_a_gate_when_the_log_is_read_then_the_state_says_so_and_the_gate_count_agrees()
 {
    let gate = Gate::new();
    let client = mock();
    client.push_reply(gated(&gate));
    let request_value = request();
    let mut sink = RecordingSink::new();
    let mut call = pin!(client.call(&request_value, &mut sink));
    assert!(poll_once(&mut call).is_pending());
    assert_eq!(gate.waiting(), 1);
    assert_eq!(kinds(&client), [StateKind::WaitingAtGate]);
}
