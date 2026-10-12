use std::pin::pin;
use std::sync::Arc;
use std::task::Poll;

use br_llm_clients_core::{Client, StreamSink};
use br_llm_messages::{BlockKind, StopReason, StreamEvent};

use crate::chunking::Chunking;
use crate::derive_events::derive_events;
use crate::recording_sink::RecordingSink;
use crate::reply::Reply;
use crate::test_support::*;

fn event(index: usize) -> StreamEvent {
    StreamEvent::BlockEnd { index }
}

fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn given_the_sink_when_compiled_then_it_is_send_sync_and_usable_as_a_trait_object() {
    assert_send_sync::<RecordingSink>();
    let boxed: Box<dyn StreamSink> = Box::new(RecordingSink::new());
    drop(boxed);
}

#[test]
fn given_events_when_sent_then_they_are_kept_in_the_order_received() {
    let mut sink = RecordingSink::new();
    for index in 0..5 {
        block_on(sink.send(event(index)));
    }
    assert_eq!(sink.events(), (0..5).map(event).collect::<Vec<_>>());
    assert_eq!(sink.len(), 5);
    assert!(!sink.is_empty());
}

#[test]
fn given_a_send_when_polled_once_then_it_is_ready_at_once_so_a_sink_is_always_ready() {
    let mut sink = RecordingSink::new();
    let mut future = pin!(sink.send(event(0)));
    assert!(matches!(poll_once(&mut future), Poll::Ready(())));
}

#[test]
fn given_a_send_future_that_is_never_polled_when_dropped_then_nothing_is_recorded() {
    let mut sink = RecordingSink::new();
    drop(sink.send(event(0)));
    assert!(sink.is_empty());
}

#[test]
fn given_a_clone_when_events_are_sent_through_one_handle_then_the_other_reads_them_at_once() {
    let reader = RecordingSink::new();
    let mut writer = reader.clone();
    block_on(writer.send(event(1)));
    assert_eq!(reader.events(), [event(1)]);
}

#[test]
fn given_take_when_called_then_it_returns_the_events_and_empties_the_sink() {
    let mut sink = RecordingSink::new();
    block_on(sink.send(event(0)));
    block_on(sink.send(event(1)));
    assert_eq!(sink.take(), [event(0), event(1)]);
    assert!(sink.is_empty());
    assert_eq!(sink.events(), []);
}

#[test]
fn given_a_sink_used_for_two_calls_when_read_then_it_holds_the_events_of_both_in_order() {
    let client = mock();
    client.push_reply(Reply::success(text_step("one")).build().unwrap());
    client.push_reply(Reply::success(text_step("two")).build().unwrap());
    let mut sink = RecordingSink::new();
    block_on(client.call(&request(), &mut sink));
    block_on(client.call(&request(), &mut sink));
    let single = derive_events(&text_step("one"), &Chunking::none())
        .unwrap()
        .into_events();
    assert_eq!(sink.len(), single.len() * 2);
}

#[test]
fn given_the_events_of_a_call_when_folded_with_a_model_then_the_step_comes_back() {
    let client = mock();
    client.push_reply(Reply::success(calls_step(&["a"])).build().unwrap());
    let (outcome, sink) = run(&client, &request());
    let folded = sink.fold(Some(model("settings-model"))).unwrap();
    assert_eq!(folded, step_of(&outcome));
}

#[test]
fn given_events_that_do_not_fold_when_folded_then_an_error_with_a_reason() {
    let mut sink = RecordingSink::new();
    block_on(sink.send(StreamEvent::TextDelta {
        index: 0,
        text: "orphan".to_owned(),
    }));
    let error = sink.fold(None).unwrap_err();
    assert!(!error.reason().is_empty());
}

#[test]
fn given_no_finish_event_when_folded_then_an_error() {
    let mut sink = RecordingSink::new();
    block_on(sink.send(StreamEvent::BlockStart {
        index: 0,
        kind: BlockKind::Text,
    }));
    block_on(sink.send(StreamEvent::BlockEnd { index: 0 }));
    assert!(sink.fold(None).is_err());
    block_on(sink.send(StreamEvent::Finish {
        stop_reason: StopReason::EndTurn,
        usage: None,
    }));
    assert!(sink.fold(None).is_err());
}

#[test]
fn given_a_wait_for_events_when_events_arrive_then_the_waker_is_woken_and_the_wait_ends() {
    let sink = RecordingSink::new();
    let wakes = Arc::new(WakeCount::default());
    let waker = wakes.waker();
    let mut wait = pin!(sink.wait_for_events(2));
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
    let mut zero = pin!(sink.wait_for_events(0));
    assert!(matches!(poll_once(&mut zero), Poll::Ready(())));
    let mut pending = Box::pin(sink.wait_for_events(9));
    assert!(poll_once(&mut pending.as_mut()).is_pending());
    drop(pending);
    let mut writer = sink.clone();
    block_on(writer.send(event(0)));
    assert_eq!(sink.len(), 1);
}

#[test]
fn given_a_reader_on_another_thread_when_a_call_streams_then_the_reader_sees_events_while_the_call_runs()
 {
    let gate = crate::gate::Gate::new();
    let client = mock();
    client.push_reply(
        Reply::success(text_step("hello"))
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
    block_on(sink.wait_for_events(2));
    assert_eq!(sink.len(), 2);
    gate.open();
    assert!(caller.join().unwrap().result().is_ok());
    assert!(sink.len() > 2);
}
