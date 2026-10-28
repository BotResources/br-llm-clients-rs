use std::sync::{Arc, Mutex, OnceLock};

use br_llm_clients_core::IdSource;
use br_llm_messages::BlockId;

use crate::MockClient;
use crate::identity_support::ids_in_order;
use crate::reply::Reply;
use crate::sequential_id_source::{SequentialIdSource, identity_uuid};
use crate::test_support::*;

const START: u64 = 100;

fn client_with(source: impl IdSource + 'static) -> MockClient {
    MockClient::builder(settings(), everything_caps())
        .id_source(source)
        .build()
}

#[test]
fn given_a_clone_and_a_handle_with_other_settings_when_called_then_all_draw_identities_from_the_source_of_the_original()
 {
    let source = SequentialIdSource::starting_at(START);
    let original = client_with(source.clone());
    original.set_fallback(Reply::text("said").build().unwrap());
    let cloned = original.clone();
    let other = original.with_settings(settings_with(
        br_llm_clients_core::ReasoningLevel::Low,
        br_llm_clients_core::CacheRequirement::NotRequired,
        500,
    ));
    let from_original = response_from(&run(&original, &request()).0);
    let from_clone = response_from(&run(&cloned, &request()).0);
    let from_other = response_from(&run(&other, &request()).0);
    assert_eq!(ids_in_order(&from_original), [identity_uuid(START)]);
    assert_eq!(ids_in_order(&from_clone), [identity_uuid(START + 1)]);
    assert_eq!(ids_in_order(&from_other), [identity_uuid(START + 2)]);
    assert_eq!(source.given(), 3);
}

struct ReentrantSource {
    inner: SequentialIdSource,
    client: Arc<OnceLock<MockClient>>,
    seen: Arc<Mutex<Vec<(usize, usize, usize)>>>,
}

impl IdSource for ReentrantSource {
    fn next_block_id(&self) -> BlockId {
        if let Some(client) = self.client.get() {
            let counts = (client.log().len(), client.calls(), client.unused_entries());
            client.push_reply(Reply::text("pushed from the source").build().unwrap());
            if let Ok(mut seen) = self.seen.lock() {
                seen.push(counts);
            }
        }
        self.inner.next_block_id()
    }
}

#[test]
fn given_a_source_that_uses_the_same_mock_while_it_runs_when_a_call_is_served_then_no_lock_is_held_and_the_mock_stays_usable()
 {
    let client_cell = Arc::new(OnceLock::new());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let client = client_with(ReentrantSource {
        inner: SequentialIdSource::starting_at(START),
        client: Arc::clone(&client_cell),
        seen: Arc::clone(&seen),
    });
    assert!(client_cell.set(client.clone()).is_ok());
    client.push_reply(Reply::text("outer").build().unwrap());
    let (outcome, _) = run(&client, &request());
    assert_eq!(text_of(&outcome), "outer");
    assert_eq!(*seen.lock().unwrap(), [(1, 1, 0)]);
    let (next, _) = run(&client, &request());
    assert_eq!(text_of(&next), "pushed from the source");
}
