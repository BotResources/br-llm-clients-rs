use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Mutex;
use std::sync::mpsc;

use br_llm_clients_core::{BoxFuture, Client, StreamSink};
use br_llm_messages::StreamEvent;

use crate::call_log::{CallState, Served, StateKind};
use crate::mock_client::MockClient;
use crate::reply::Reply;
use crate::reply_source::ReplySource;
use crate::rule::Rule;
use crate::test_support::*;

fn says(value: &str) -> Reply {
    Reply::success(text_step(value)).build().unwrap()
}

struct ReentrantSink {
    client: MockClient,
    inner_answers: Vec<String>,
}

impl StreamSink for ReentrantSink {
    fn send<'a>(&'a mut self, event: StreamEvent) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            if !matches!(event, StreamEvent::BlockStart { .. }) {
                return;
            }
            let log_len = self.client.log().len();
            self.client.push_reply(says("pushed from the sink"));
            self.client.set_fallback(says("fallback from the sink"));
            let _ = (
                self.client.calls(),
                self.client.unused_entries(),
                self.client.unused_uses(),
            );
            let (inner, _) = run(&self.client, &request());
            self.inner_answers.push(format!(
                "{log_len}:{}",
                inner
                    .result()
                    .map(|found| found.text().next().unwrap().to_string())
                    .unwrap_or_default()
            ));
        })
    }
}

#[test]
fn given_a_sink_that_uses_the_same_mock_while_it_runs_when_a_call_streams_then_there_is_no_deadlock()
 {
    let client = mock();
    client.push_reply(says("outer"));
    let mut sink = ReentrantSink {
        client: client.clone(),
        inner_answers: Vec::new(),
    };
    let outcome = block_on(client.call(&request(), &mut sink));
    assert!(outcome.result().is_ok());
    assert_eq!(sink.inner_answers, ["1:pushed from the sink"]);
    assert_eq!(client.calls(), 2);
}

#[test]
fn given_a_reply_function_that_uses_the_same_mock_when_it_runs_then_there_is_no_deadlock() {
    let client = mock();
    let inside = client.clone();
    client.push_reply(ReplySource::function(move |_| {
        inside.push_reply(says("pushed from the function"));
        let seen = inside.log().len();
        Reply::success(text_step(&format!("function saw {seen}"))).build()
    }));
    let (outcome, _) = run(&client, &request());
    assert_eq!(
        step_of(&outcome).text().next().unwrap().as_str(),
        "function saw 1"
    );
    let (next, _) = run(&client, &request());
    assert_eq!(
        step_of(&next).text().next().unwrap().as_str(),
        "pushed from the function"
    );
}

#[test]
fn given_a_matcher_that_blocks_when_the_test_thread_uses_the_mock_then_no_lock_is_held_and_the_log_shows_not_yet_claimed()
 {
    let (entered_tx, entered_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::channel::<()>();
    let resume_rx = Mutex::new(resume_rx);
    let client = mock();
    client.push_rule(Rule::new(
        move |_| {
            entered_tx.send(()).unwrap();
            resume_rx.lock().unwrap().recv().unwrap();
            false
        },
        says("never"),
    ));
    client.push_reply(says("entry"));
    let caller = {
        let client = client.clone();
        std::thread::spawn(move || run(&client, &request()).0)
    };
    entered_rx.recv().unwrap();
    let log = client.log();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].served(), &Served::NotYetClaimed);
    assert_eq!(log[0].state().kind(), StateKind::Running);
    assert_eq!(client.calls(), 1);
    client.push_reply(says("added meanwhile"));
    assert_eq!(client.unused_entries(), 2);
    resume_tx.send(()).unwrap();
    let outcome = caller.join().unwrap();
    assert_eq!(step_of(&outcome).text().next().unwrap().as_str(), "entry");
}

#[test]
fn given_a_reply_function_that_blocks_when_other_calls_are_made_then_they_are_not_blocked() {
    let (entered_tx, entered_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::channel::<()>();
    let resume_rx = Mutex::new(resume_rx);
    let client = mock();
    client.push_reply(ReplySource::function(move |_| {
        entered_tx.send(()).unwrap();
        resume_rx.lock().unwrap().recv().unwrap();
        Ok(says("slow function"))
    }));
    client.set_fallback(says("free"));
    let caller = {
        let client = client.clone();
        std::thread::spawn(move || run(&client, &request()).0)
    };
    entered_rx.recv().unwrap();
    let (free, _) = run(&client, &request());
    assert_eq!(step_of(&free).text().next().unwrap().as_str(), "free");
    resume_tx.send(()).unwrap();
    assert!(caller.join().unwrap().result().is_ok());
}

struct PanickingSink;

impl StreamSink for PanickingSink {
    fn send<'a>(&'a mut self, _event: StreamEvent) -> BoxFuture<'a, ()> {
        Box::pin(async move { panic!("the sink failed") })
    }
}

#[test]
fn given_a_panic_in_a_sink_when_the_call_is_polled_then_the_panic_goes_to_the_poller_the_call_is_dropped_and_the_mock_stays_usable()
 {
    let client = mock();
    client.push_reply(says("taken by the panic"));
    client.push_reply(says("next"));
    let request_value = request();
    let mut sink = PanickingSink;
    let mut call = Box::pin(client.call(&request_value, &mut sink));
    let panic = catch_unwind(AssertUnwindSafe(|| {
        let _ = poll_once(&mut call.as_mut());
    }));
    assert!(panic.is_err());
    assert_eq!(client.log()[0].state(), &CallState::Dropped);
    assert_eq!(client.unused_entries(), 1);
    drop(call);
    let (next, _) = run(&client, &request());
    assert_eq!(step_of(&next).text().next().unwrap().as_str(), "next");
    assert_eq!(client.log().len(), 2);
}

#[test]
fn given_a_panic_in_a_matcher_when_the_call_is_polled_then_no_entry_and_no_use_is_taken_and_the_log_says_nothing_served()
 {
    let client = mock();
    client.push_rule(
        Rule::new(
            |call| {
                if call.sequence() == 0 {
                    panic!("the matcher failed");
                }
                false
            },
            says("never"),
        )
        .with_limit(3)
        .unwrap(),
    );
    client.push_reply(says("entry"));
    let request_value = request();
    let mut sink = crate::RecordingSink::new();
    let mut call = Box::pin(client.call(&request_value, &mut sink));
    let panic = catch_unwind(AssertUnwindSafe(|| {
        let _ = poll_once(&mut call.as_mut());
    }));
    assert!(panic.is_err());
    let log = client.log();
    assert_eq!(log[0].served(), &Served::Nothing);
    assert_eq!(log[0].state(), &CallState::Dropped);
    assert_eq!(client.unused_entries(), 1);
    assert_eq!(client.unused_uses(), 3);
    drop(call);
    let (next, _) = run(&client, &request());
    assert_eq!(step_of(&next).text().next().unwrap().as_str(), "entry");
}

#[test]
fn given_a_panic_in_a_reply_function_when_the_call_is_polled_then_the_entry_stays_taken_and_the_call_is_dropped()
 {
    let client = mock();
    client.push_reply(ReplySource::function(|_| panic!("the function failed")));
    client.push_reply(says("next"));
    let request_value = request();
    let mut sink = crate::RecordingSink::new();
    let mut call = Box::pin(client.call(&request_value, &mut sink));
    let panic = catch_unwind(AssertUnwindSafe(|| {
        let _ = poll_once(&mut call.as_mut());
    }));
    assert!(panic.is_err());
    assert_eq!(client.log()[0].served(), &Served::Entry { position: 0 });
    assert_eq!(client.log()[0].state(), &CallState::Dropped);
    assert_eq!(client.unused_entries(), 1);
    drop(call);
    let (next, _) = run(&client, &request());
    assert!(next.result().is_ok());
}

#[test]
fn given_a_panic_in_a_rule_reply_function_when_the_call_is_polled_then_the_use_stays_taken() {
    let client = mock();
    client.push_rule(
        Rule::always(ReplySource::function(|_| panic!("the function failed")))
            .with_limit(2)
            .unwrap(),
    );
    let request_value = request();
    let mut sink = crate::RecordingSink::new();
    let mut call = Box::pin(client.call(&request_value, &mut sink));
    let panic = catch_unwind(AssertUnwindSafe(|| {
        let _ = poll_once(&mut call.as_mut());
    }));
    assert!(panic.is_err());
    assert_eq!(client.unused_uses(), 1);
    assert_eq!(client.log()[0].state(), &CallState::Dropped);
}

#[test]
fn given_a_panic_in_user_code_on_one_thread_when_other_threads_use_the_mock_then_they_are_unaffected()
 {
    let client = mock();
    client.set_fallback(says("fine"));
    client.push_rule(Rule::new(
        |call| {
            if call.sequence() == 0 {
                panic!("only the first call fails");
            }
            false
        },
        says("never"),
    ));
    let failing = {
        let client = client.clone();
        std::thread::spawn(move || run(&client, &request()))
    };
    assert!(failing.join().is_err());
    for _ in 0..4 {
        let (outcome, _) = run(&client, &request());
        assert_eq!(step_of(&outcome).text().next().unwrap().as_str(), "fine");
    }
    assert_eq!(client.calls(), 5);
    assert_eq!(
        client
            .log()
            .iter()
            .filter(|call| call.state().kind() == StateKind::Succeeded)
            .count(),
        4
    );
}
