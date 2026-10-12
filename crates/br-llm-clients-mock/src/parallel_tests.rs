use std::collections::BTreeSet;
use std::pin::pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::task::Poll;

use br_llm_clients_core::{CallOutcome, Client, ClientError};

use crate::call_log::{CallState, Served, StateKind};
use crate::capabilities::Capabilities;
use crate::gate::Gate;
use crate::mock_client::MockClient;
use crate::recording_sink::RecordingSink;
use crate::reply::Reply;
use crate::rule::Rule;
use crate::test_support::*;

const PARALLEL: usize = 1000;

fn says(value: &str) -> Reply {
    Reply::success(text_step(value)).build().unwrap()
}

fn text_of(outcome: &CallOutcome) -> String {
    step_of(outcome).text().next().unwrap().to_string()
}

fn spawn_small<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> std::thread::JoinHandle<T> {
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(work)
        .unwrap()
}

fn assert_send<T: Send>(_: &T) {}

#[test]
fn given_the_mock_when_compiled_then_it_is_a_trait_object_and_its_call_future_is_send() {
    let shared: Arc<dyn Client> = Arc::new(mock());
    let request_value = request();
    let mut sink = RecordingSink::new();
    let future = shared.call(&request_value, &mut sink);
    assert_send(&future);
}

#[test]
fn given_a_thousand_tests_in_parallel_each_with_its_own_mock_when_run_then_each_gets_its_own_answers()
 {
    let barrier = Arc::new(Barrier::new(PARALLEL));
    let handles: Vec<_> = (0..PARALLEL)
        .map(|number| {
            let barrier = Arc::clone(&barrier);
            spawn_small(move || {
                let client = mock();
                client.push_reply(says(&format!("answer {number}")));
                client.push_reply(Reply::failure(ClientError::Transport).build().unwrap());
                barrier.wait();
                let (first, sink) = run(&client, &request());
                let (second, _) = run(&client, &request());
                (
                    number,
                    text_of(&first),
                    error_of(&second),
                    sink.len(),
                    client.calls(),
                )
            })
        })
        .collect();
    for handle in handles {
        let (number, answer, error, events, calls) = handle.join().unwrap();
        assert_eq!(answer, format!("answer {number}"));
        assert_eq!(error, ClientError::Transport);
        assert!(events > 1);
        assert_eq!(calls, 2);
    }
}

#[test]
fn given_one_shared_mock_called_by_a_thousand_threads_when_run_then_every_entry_is_claimed_exactly_once()
 {
    let client = mock();
    for number in 0..PARALLEL {
        client.push_reply(says(&format!("entry {number}")));
    }
    let barrier = Arc::new(Barrier::new(PARALLEL));
    let handles: Vec<_> = (0..PARALLEL)
        .map(|_| {
            let (client, barrier) = (client.clone(), Arc::clone(&barrier));
            spawn_small(move || {
                barrier.wait();
                text_of(&run(&client, &request()).0)
            })
        })
        .collect();
    let answers: BTreeSet<String> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    assert_eq!(answers.len(), PARALLEL);
    assert_eq!(client.unused_entries(), 0);
    assert_eq!(client.calls(), PARALLEL);
    let log = client.log();
    assert_eq!(log.len(), PARALLEL);
    for (position, call) in log.iter().enumerate() {
        assert_eq!(call.sequence(), position);
        assert!(matches!(call.state(), CallState::Succeeded(_)));
        assert!(matches!(call.served(), Served::Entry { .. }));
    }
}

#[test]
fn given_one_shared_mock_with_a_limited_rule_when_called_in_parallel_then_the_limit_is_never_exceeded()
 {
    let client = mock();
    client.push_rule(Rule::always(says("limited")).with_limit(100).unwrap());
    client.set_fallback(says("fallback"));
    let handles: Vec<_> = (0..PARALLEL)
        .map(|_| {
            let client = client.clone();
            spawn_small(move || text_of(&run(&client, &request()).0))
        })
        .collect();
    let answers: Vec<String> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    assert_eq!(
        answers.iter().filter(|answer| *answer == "limited").count(),
        100
    );
    assert_eq!(client.unused_uses(), 0);
}

#[test]
fn given_two_calls_that_both_match_the_last_use_when_they_claim_together_then_one_goes_on_to_the_next_rule()
 {
    let rendezvous = Arc::new(Barrier::new(2));
    let waiting = Arc::clone(&rendezvous);
    let client = mock();
    client.push_rule(
        Rule::new(
            move |_| {
                waiting.wait();
                true
            },
            says("first rule"),
        )
        .with_limit(1)
        .unwrap(),
    );
    client.push_rule(Rule::always(says("second rule")));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let client = client.clone();
            spawn_small(move || text_of(&run(&client, &request()).0))
        })
        .collect();
    let mut answers: Vec<String> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    answers.sort();
    assert_eq!(answers, ["first rule", "second rule"]);
}

#[test]
fn given_a_hung_call_when_other_calls_are_made_on_the_same_mock_then_they_are_not_blocked() {
    let client = mock();
    client.push_reply(Reply::hang(text_step("stuck"), 1).build().unwrap());
    client.set_fallback(says("free"));
    let request_value = request();
    let mut hung_sink = RecordingSink::new();
    let mut hung = Box::pin(client.call(&request_value, &mut hung_sink));
    assert!(poll_once(&mut hung.as_mut()).is_pending());
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let client = client.clone();
            spawn_small(move || text_of(&run(&client, &request()).0))
        })
        .collect();
    for handle in handles {
        assert_eq!(handle.join().unwrap(), "free");
    }
    assert_eq!(client.log()[0].state().kind(), StateKind::Hanging);
}

#[test]
fn given_a_gate_wait_in_one_task_when_another_task_calls_the_same_mock_then_it_is_not_blocked() {
    let gate = Gate::new();
    let client = mock();
    client.push_reply(
        Reply::success(text_step("held"))
            .gate_at(0, &gate)
            .build()
            .unwrap(),
    );
    client.set_fallback(says("free"));
    let request_value = request();
    let mut held_sink = RecordingSink::new();
    let mut held = Box::pin(client.call(&request_value, &mut held_sink));
    assert!(poll_once(&mut held.as_mut()).is_pending());
    assert_eq!(text_of(&run(&client, &request()).0), "free");
}

#[test]
fn given_two_mocks_when_one_is_used_then_the_other_is_untouched() {
    let first = mock();
    let second = mock();
    first.push_reply(says("only first"));
    second.push_reply(says("only second"));
    run(&first, &request());
    assert_eq!(second.calls(), 0);
    assert_eq!(second.unused_entries(), 1);
    assert_eq!(text_of(&run(&second, &request()).0), "only second");
}

#[test]
fn given_the_same_mock_and_the_same_calls_in_the_same_order_when_repeated_then_the_outcomes_are_equal()
 {
    let build = || {
        let client = mock();
        client.push_reply(says("a"));
        client.push_reply(Reply::failure(ClientError::Transport).build().unwrap());
        client.push_rule(Rule::new(|call| call.sequence() == 2, says("rule")));
        client.set_fallback(says("fallback"));
        client
    };
    let outcomes = |client: &MockClient| -> Vec<CallOutcome> {
        (0..4).map(|_| run(client, &request()).0).collect()
    };
    assert_eq!(outcomes(&build()), outcomes(&build()));
}

#[test]
fn given_many_sequential_calls_on_a_mock_without_log_when_made_then_all_complete_and_counts_stay_exact()
 {
    let client = MockClient::builder(settings(), Capabilities::everything())
        .log(false)
        .fallback(says("again"))
        .build();
    let request_value = request();
    let completed = AtomicUsize::new(0);
    for _ in 0..20_000 {
        let mut sink = RecordingSink::new();
        let mut call = pin!(client.call(&request_value, &mut sink));
        if matches!(poll_once(&mut call), Poll::Ready(_)) {
            completed.fetch_add(1, Ordering::SeqCst);
        }
    }
    assert_eq!(completed.load(Ordering::SeqCst), 20_000);
    assert_eq!(client.calls(), 20_000);
}
