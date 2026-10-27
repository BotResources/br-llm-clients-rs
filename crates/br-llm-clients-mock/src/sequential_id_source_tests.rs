use std::collections::HashSet;
use std::sync::Arc;

use br_llm_clients_core::IdSource;
use br_llm_messages::Uuid;

use crate::sequential_id_source::{DEFAULT_ID_START, SequentialIdSource, identity_uuid};
use crate::test_support::*;
use crate::{MockClient, Reply};

const LAST_NUMBER: u64 = (1 << 62) - 1;

fn uuid_of(id: br_llm_messages::BlockId) -> Uuid {
    id.as_uuid()
}

#[test]
fn given_a_new_source_when_asked_then_identity_zero_is_the_uuid_of_number_one_in_version_7_and_rfc_4122()
 {
    let source = SequentialIdSource::new();
    let first = uuid_of(source.next_block_id());
    assert_eq!(first.to_string(), "00000000-0000-7000-8000-000000000001");
    assert_eq!(first.get_version_num(), 7);
    let second = uuid_of(source.next_block_id());
    assert_eq!(second.to_string(), "00000000-0000-7000-8000-000000000002");
}

#[test]
fn given_a_source_starting_at_s_when_asked_then_the_kth_identity_is_the_identity_number_s_plus_k() {
    let start = SequentialIdSource::starting_at(40);
    let reference = SequentialIdSource::new();
    for _ in 0..40 {
        reference.next_block_id();
    }
    for _ in 0..3 {
        assert_eq!(start.next_block_id(), reference.next_block_id());
    }
    assert_eq!(uuid_of(start.next_block_id()), identity_uuid(43));
}

#[test]
fn given_a_source_when_read_then_the_number_is_absolute_and_reading_takes_nothing() {
    let source = SequentialIdSource::starting_at(10);
    assert_eq!(source.next_number(), 10);
    assert_eq!(source.next_number(), 10);
    assert_eq!(source.given(), 0);
    source.next_block_id();
    source.next_block_id();
    assert_eq!(source.next_number(), 12);
    assert_eq!(source.given(), 2);
}

#[test]
fn given_a_clone_when_asked_then_it_shares_the_count_and_a_new_source_starts_again() {
    let source = SequentialIdSource::new();
    let handle = source.clone();
    let first = source.next_block_id();
    let second = handle.next_block_id();
    assert_ne!(first, second);
    assert_eq!(source.given(), 2);
    assert_eq!(handle.next_number(), 2);
    assert_eq!(SequentialIdSource::new().next_block_id(), first);
}

#[test]
fn given_entry_and_turn_ids_when_taken_then_they_come_from_the_same_count() {
    let source = SequentialIdSource::new();
    let entry = source.next_entry_id().as_uuid();
    let block = uuid_of(source.next_block_id());
    let turn = source.next_turn_id().as_uuid();
    assert_eq!(entry, identity_uuid(0));
    assert_eq!(block, identity_uuid(1));
    assert_eq!(turn, identity_uuid(2));
    assert_eq!(source.given(), 3);
}

#[test]
fn given_many_threads_when_asking_at_once_then_every_identity_is_distinct_and_counted() {
    let source = SequentialIdSource::new();
    let ids: Vec<Vec<_>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|_| scope.spawn(|| (0..200).map(|_| source.next_block_id()).collect::<Vec<_>>()))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let distinct: HashSet<_> = ids.into_iter().flatten().collect();
    assert_eq!(distinct.len(), 1600);
    assert_eq!(source.given(), 1600);
}

#[test]
fn given_a_source_at_the_last_distinct_number_when_asked_then_it_gives_it_and_the_next_call_panics()
{
    let source = SequentialIdSource::starting_at(LAST_NUMBER - 1);
    let last = source.next_block_id();
    assert_eq!(uuid_of(last), identity_uuid(LAST_NUMBER - 1));
    assert_eq!(uuid_of(last).get_version_num(), 7);
    assert_eq!(uuid_of(last).as_bytes()[8] & 0xC0, 0x80);
    let again = std::panic::catch_unwind(|| source.next_block_id());
    assert!(again.is_err());
    assert_eq!(source.given(), 1);
}

#[test]
fn given_a_source_starting_at_or_above_the_limit_when_asked_then_it_panics_and_repeats_nothing() {
    for start in [LAST_NUMBER, u64::MAX] {
        let source = SequentialIdSource::starting_at(start);
        assert!(std::panic::catch_unwind(|| source.next_block_id()).is_err());
        assert!(std::panic::catch_unwind(|| source.next_entry_id()).is_err());
        assert!(std::panic::catch_unwind(|| source.next_turn_id()).is_err());
    }
}

#[test]
fn given_the_default_start_when_read_then_it_is_two_to_the_sixty_one() {
    assert_eq!(DEFAULT_ID_START, 1u64 << 61);
}

#[test]
fn given_the_source_as_a_trait_object_when_shared_across_threads_then_it_compiles_and_works() {
    let source: Arc<dyn IdSource> = Arc::new(SequentialIdSource::new());
    let handle = Arc::clone(&source);
    let id = std::thread::spawn(move || handle.next_block_id())
        .join()
        .unwrap();
    assert_eq!(uuid_of(id), identity_uuid(0));
}

#[test]
fn given_two_mocks_without_a_source_when_serving_the_same_calls_then_the_identities_are_the_same_and_start_at_the_default()
 {
    let serve = || {
        let client = mock();
        client.push_reply(Reply::text("hi").build().unwrap());
        let request_value = request();
        response_from(&run(&client, &request_value).0)
    };
    let first = serve();
    assert_eq!(first, serve());
    assert_eq!(
        uuid_of(first.blocks()[0].id()),
        identity_uuid(DEFAULT_ID_START)
    );
}

#[test]
fn given_a_clone_given_to_two_mocks_when_both_serve_then_the_identities_do_not_repeat() {
    let source = SequentialIdSource::starting_at(5000);
    let build = || {
        MockClient::builder(settings(), everything_caps())
            .id_source(source.clone())
            .build()
    };
    let (first, second) = (build(), build());
    first.push_reply(Reply::text("a").build().unwrap());
    second.push_reply(Reply::text("b").build().unwrap());
    let one = response_from(&run(&first, &request()).0);
    let two = response_from(&run(&second, &request()).0);
    assert_eq!(uuid_of(one.blocks()[0].id()), identity_uuid(5000));
    assert_eq!(uuid_of(two.blocks()[0].id()), identity_uuid(5001));
}
