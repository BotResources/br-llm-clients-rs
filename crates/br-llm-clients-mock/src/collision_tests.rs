use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use br_llm_clients_core::{Client, ClientError, IdSource};
use br_llm_messages::{BlockId, ResponseEnding};

use crate::MockClient;
use crate::call_log::{CallState, Served};
use crate::codes::CODE_IDENTITY_COLLISION;
use crate::recording_sink::RecordingSink;
use crate::reply::Reply;
use crate::response_support::*;
use crate::sequential_id_source::{SequentialIdSource, identity_uuid};
use crate::test_support::*;

#[derive(Clone)]
struct Constant {
    id: BlockId,
    draws: Arc<AtomicUsize>,
}

impl Constant {
    fn new() -> Self {
        Self {
            id: BlockId::from_uuid(identity_uuid(7)).unwrap(),
            draws: Arc::new(AtomicUsize::new(0)),
        }
    }
}

impl IdSource for Constant {
    fn next_block_id(&self) -> BlockId {
        self.draws.fetch_add(1, Ordering::SeqCst);
        self.id
    }
}

struct PanicsOnce {
    inner: SequentialIdSource,
    armed: Arc<AtomicUsize>,
}

impl IdSource for PanicsOnce {
    fn next_block_id(&self) -> BlockId {
        if self.armed.fetch_add(1, Ordering::SeqCst) == 0 {
            panic!("the source of identities panics");
        }
        self.inner.next_block_id()
    }
}

fn two_texts() -> br_llm_messages::ModelResponse {
    response_ending(
        vec![text_content("a"), text_content("b")],
        ResponseEnding::finished(),
    )
}

fn collision_mock(source: Constant) -> MockClient {
    MockClient::builder(settings(), everything_caps())
        .id_source(source)
        .build()
}

#[test]
fn given_a_source_that_repeats_an_identity_when_a_response_is_served_then_identity_collision_with_a_bare_report()
 {
    let source = Constant::new();
    let client = collision_mock(source.clone());
    client.push_reply(Reply::success(two_texts()).build().unwrap());
    let (outcome, sink) = run(&client, &request());
    let ClientError::ProviderRejected { status, code, body } = error_of(&outcome) else {
        panic!("provider rejected expected");
    };
    assert_eq!(status, None);
    assert_eq!(code.as_deref(), Some(CODE_IDENTITY_COLLISION));
    assert!(!body.is_empty());
    assert!(sink.is_empty());
    let report = outcome.report();
    assert_eq!(report.total_duration(), Duration::ZERO);
    assert_eq!(report.usage(), None);
    assert_eq!(report.model(), None);
    assert_eq!(report.request_id(), None);
    assert_eq!(report.time_to_first_event(), None);
}

#[test]
fn given_a_collision_when_served_then_the_identities_drawn_stay_taken_and_the_entry_stays_taken() {
    let source = Constant::new();
    let client = collision_mock(source.clone());
    client.push_reply(Reply::success(two_texts()).build().unwrap());
    client.push_reply(Reply::text("next").build().unwrap());
    run(&client, &request());
    assert_eq!(source.draws.load(Ordering::SeqCst), 2);
    assert_eq!(client.unused_entries(), 1);
    assert_eq!(client.log()[0].served(), &Served::Entry { position: 0 });
}

#[test]
fn given_a_collision_when_a_cut_or_a_hang_is_served_then_it_is_the_same_error_with_no_delta() {
    for reply in [
        Reply::cut(two_texts(), 1, ClientError::Transport)
            .build()
            .unwrap(),
        Reply::hang(two_texts(), 1).build().unwrap(),
    ] {
        let client = collision_mock(Constant::new());
        client.push_reply(reply);
        let (outcome, sink) = run(&client, &request());
        assert!(matches!(
            error_of(&outcome),
            ClientError::ProviderRejected { code: Some(code), .. } if code == CODE_IDENTITY_COLLISION
        ));
        assert!(sink.is_empty());
    }
}

#[test]
fn given_a_collision_and_a_response_the_request_would_refuse_when_served_then_the_collision_comes_first()
 {
    let structured_in_plain = response_ending(
        vec![structured_content("1"), text_content("t")],
        ResponseEnding::finished(),
    );
    let client = collision_mock(Constant::new());
    client.push_reply(Reply::success(structured_in_plain).build().unwrap());
    let (outcome, _) = run(&client, &plain_request());
    assert!(matches!(
        error_of(&outcome),
        ClientError::ProviderRejected { .. }
    ));
}

#[test]
fn given_a_response_with_one_block_when_the_source_always_gives_the_same_identity_then_there_is_no_collision()
 {
    let client = collision_mock(Constant::new());
    client.push_reply(Reply::text("one").build().unwrap());
    assert!(run(&client, &request()).0.result().is_ok());
}

#[test]
fn given_a_collision_between_a_citation_and_a_block_when_served_then_identity_collision() {
    let client = collision_mock(Constant::new());
    let built = crate::maximal::maximal();
    client.push_reply(Reply::success(built.response).build().unwrap());
    let (outcome, _) = run(&client, &request());
    assert!(matches!(
        error_of(&outcome),
        ClientError::ProviderRejected { code: Some(code), .. } if code == CODE_IDENTITY_COLLISION
    ));
}

#[test]
fn given_a_collision_when_logged_then_the_call_finished_with_the_error_and_the_mock_is_usable() {
    let client = collision_mock(Constant::new());
    client.push_reply(Reply::success(two_texts()).build().unwrap());
    let (outcome, _) = run(&client, &request());
    assert_eq!(client.log()[0].state(), &CallState::Failed(outcome));
    client.set_fallback(Reply::failure(ClientError::Transport).build().unwrap());
    assert_eq!(
        error_of(&run(&client, &request()).0),
        ClientError::Transport
    );
}

#[test]
fn given_a_source_that_panics_when_a_call_serves_a_response_then_the_panic_goes_to_the_poller_and_the_call_is_dropped()
 {
    let armed = Arc::new(AtomicUsize::new(0));
    let source = PanicsOnce {
        inner: SequentialIdSource::new(),
        armed: Arc::clone(&armed),
    };
    let client = MockClient::builder(settings(), everything_caps())
        .id_source(source)
        .build();
    client.push_reply(Reply::text("first").build().unwrap());
    client.push_reply(Reply::text("second").build().unwrap());
    let request_value = request();
    let mut sink = RecordingSink::new();
    let mut call = Box::pin(client.call(&request_value, &mut sink));
    let polled = catch_unwind(AssertUnwindSafe(|| poll_once(&mut call.as_mut())));
    assert!(polled.is_err());
    drop(call);
    assert_eq!(client.log()[0].state(), &CallState::Dropped);
    assert_eq!(client.unused_entries(), 1);
    assert_eq!(text_of(&run(&client, &request()).0), "second");
}

#[test]
fn given_a_source_that_gives_the_identity_of_the_entry_of_the_request_when_a_response_is_served_then_identity_collision()
 {
    let request_value = request();
    let source = Constant {
        id: BlockId::from_uuid(request_value.reply_entry().as_uuid()).unwrap(),
        draws: Arc::new(AtomicUsize::new(0)),
    };
    let client = collision_mock(source);
    client.push_reply(Reply::success(text_response("a")).build().unwrap());
    let (outcome, sink) = run(&client, &request_value);
    assert!(matches!(
        error_of(&outcome),
        ClientError::ProviderRejected { code: Some(code), .. } if code == CODE_IDENTITY_COLLISION
    ));
    assert!(sink.is_empty());
}
