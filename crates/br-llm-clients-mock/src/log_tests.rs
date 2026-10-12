use std::pin::pin;
use std::task::Poll;

use br_llm_clients_core::{CacheRequirement, Client, ClientError, OutputMode, ReasoningLevel};

use crate::call_log::{CallState, Served, StateKind};
use crate::capabilities::Capabilities;
use crate::gate::Gate;
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

fn kinds(client: &MockClient) -> Vec<StateKind> {
    client
        .log()
        .iter()
        .map(|call| call.state().kind())
        .collect()
}

#[test]
fn given_a_finished_success_when_logged_then_the_state_holds_a_copy_of_the_outcome() {
    let client = mock();
    client.push_reply(says("done"));
    let (outcome, _) = run(&client, &request());
    assert_eq!(client.log()[0].state(), &CallState::Succeeded(outcome));
}

#[test]
fn given_a_finished_error_when_logged_then_the_state_holds_a_copy_of_the_outcome() {
    let client = mock();
    client.push_reply(Reply::failure(ClientError::QuotaExceeded).build().unwrap());
    let (outcome, _) = run(&client, &request());
    assert_eq!(client.log()[0].state(), &CallState::Failed(outcome));
}

#[test]
fn given_calls_when_logged_then_the_log_is_in_sequence_order_with_copies_of_request_and_settings() {
    let client = mock();
    let other = client.with_settings(settings_with(
        ReasoningLevel::Low,
        CacheRequirement::NotRequired,
        5,
    ));
    let long = request_of(
        vec![user_text("a"), assistant_text("b"), user_text("c")],
        vec![],
        OutputMode::Plain,
    );
    run(&client, &request());
    run(&other, &long);
    let log = client.log();
    assert_eq!(
        log.iter().map(|call| call.sequence()).collect::<Vec<_>>(),
        [0, 1]
    );
    assert_eq!(log[0].request(), &request());
    assert_eq!(log[1].request(), &long);
    assert_eq!(log[0].settings(), &settings());
    assert_eq!(log[1].settings().max_output_tokens().get(), 5);
}

#[test]
fn given_a_hang_when_the_call_is_dropped_then_it_gave_no_report_and_the_log_says_dropped() {
    let client = mock();
    client.push_reply(Reply::hang(text_step("hi"), 1).build().unwrap());
    let request_value = request();
    let sink = RecordingSink::new();
    let mut handle = sink.clone();
    let mut call = Box::pin(client.call(&request_value, &mut handle));
    assert!(poll_once(&mut call.as_mut()).is_pending());
    assert_eq!(kinds(&client), [StateKind::Hanging]);
    drop(call);
    assert_eq!(kinds(&client), [StateKind::Dropped]);
    assert_eq!(client.unused_entries(), 0);
    assert_eq!(sink.len(), 1);
}

#[test]
fn given_a_hang_when_polled_again_then_it_stays_pending_and_never_ends() {
    let client = mock();
    client.push_reply(Reply::hang(text_step("hi"), 0).build().unwrap());
    let request_value = request();
    let mut sink = RecordingSink::new();
    let mut call = pin!(client.call(&request_value, &mut sink));
    for _ in 0..10 {
        assert!(poll_once(&mut call).is_pending());
    }
}

#[test]
fn given_a_finished_call_when_its_future_is_dropped_later_then_the_finished_state_stays() {
    let client = mock();
    client.push_reply(says("done"));
    let request_value = request();
    let mut sink = RecordingSink::new();
    let mut call = Box::pin(client.call(&request_value, &mut sink));
    assert!(matches!(poll_once(&mut call.as_mut()), Poll::Ready(_)));
    drop(call);
    assert_eq!(kinds(&client), [StateKind::Succeeded]);
}

#[test]
fn given_a_call_dropped_at_a_gate_when_logged_then_the_entry_stays_taken_and_state_is_dropped() {
    let gate = Gate::new();
    let client = mock();
    client.push_reply(gated(&gate));
    let request_value = request();
    let mut sink = RecordingSink::new();
    let mut call = Box::pin(client.call(&request_value, &mut sink));
    assert!(poll_once(&mut call.as_mut()).is_pending());
    assert_eq!(kinds(&client), [StateKind::WaitingAtGate]);
    drop(call);
    assert_eq!(kinds(&client), [StateKind::Dropped]);
    assert_eq!(client.unused_entries(), 0);
}

#[test]
fn given_a_refusal_and_an_exhausted_script_when_logged_then_each_shows_how_it_was_served() {
    let client = MockClient::new(settings(), Capabilities::everything());
    let bad = request_of(vec![assistant_text("x")], vec![], OutputMode::Plain);
    assert!(
        br_llm_clients_core::ClientRequest::new(
            None,
            vec![assistant_text("x")],
            vec![],
            OutputMode::Plain
        )
        .is_ok()
    );
    run(&client, &bad);
    run(&client, &request());
    let log = client.log();
    assert!(matches!(
        log[0].served(),
        Served::Refusal(ClientError::InvalidMessages { .. })
    ));
    assert_eq!(log[1].served(), &Served::Exhausted);
}

#[test]
fn given_calls_when_counted_then_the_count_follows_the_calls_that_began() {
    let client = mock();
    assert_eq!(client.calls(), 0);
    run(&client, &request());
    run(&client, &request());
    assert_eq!(client.calls(), 2);
}

#[test]
fn given_a_clone_when_it_calls_then_the_log_is_shared_with_the_original() {
    let client = mock();
    let clone = client.clone();
    run(&clone, &request());
    assert_eq!(client.calls(), 1);
    assert_eq!(client.log().len(), 1);
}
