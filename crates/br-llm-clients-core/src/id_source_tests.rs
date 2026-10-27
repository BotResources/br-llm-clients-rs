use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use br_llm_messages::BlockId;

use crate::{DefaultIdSource, IdSource};

struct Counting(AtomicU64);

impl IdSource for Counting {
    fn next_block_id(&self) -> BlockId {
        let next = self.0.fetch_add(1, Ordering::SeqCst);
        let uuid = uuid::Builder::from_u128(u128::from(next) + 1)
            .with_variant(uuid::Variant::RFC4122)
            .with_version(uuid::Version::SortRand)
            .into_uuid();
        BlockId::from_uuid(uuid).unwrap()
    }
}

fn assert_send_sync<T: Send + Sync + ?Sized>() {}

fn assert_traits<T: Default + Clone + Copy + std::fmt::Debug>() {}

#[test]
fn given_the_default_source_when_asked_many_times_then_no_identity_repeats() {
    let source = DefaultIdSource;
    let mut seen = HashSet::new();
    for _ in 0..20_000 {
        assert!(seen.insert(source.next_block_id()));
    }
}

#[test]
fn given_the_default_source_when_asked_from_several_threads_then_no_identity_repeats_across_them() {
    let source = Arc::new(DefaultIdSource);
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let source = Arc::clone(&source);
            std::thread::spawn(move || {
                (0..2_000)
                    .map(|_| source.next_block_id())
                    .collect::<Vec<_>>()
            })
        })
        .collect();
    let mut seen = HashSet::new();
    for handle in handles {
        for id in handle.join().unwrap() {
            assert!(seen.insert(id));
        }
    }
    assert_eq!(seen.len(), 16_000);
}

#[test]
fn given_two_default_sources_when_asked_then_their_identities_still_differ() {
    let (first, second) = (DefaultIdSource, DefaultIdSource);
    let mut seen = HashSet::new();
    for _ in 0..1_000 {
        assert!(seen.insert(first.next_block_id()));
        assert!(seen.insert(second.next_block_id()));
    }
}

#[test]
fn given_an_identity_of_the_default_source_when_read_then_it_is_a_uuid_version_seven() {
    let id = DefaultIdSource.next_block_id();
    assert_eq!(id.as_uuid().get_version_num(), 7);
    let again = BlockId::parse(&id.to_string()).unwrap();
    assert_eq!(again, id);
}

#[test]
fn given_the_default_source_when_compiled_then_it_is_a_unit_struct_with_the_listed_traits() {
    assert_traits::<DefaultIdSource>();
    let copy = DefaultIdSource;
    let other = copy;
    assert_eq!(
        format!("{copy:?}{other:?}"),
        "DefaultIdSourceDefaultIdSource"
    );
}

#[test]
fn given_the_trait_when_compiled_then_sources_are_shareable_and_usable_as_trait_objects() {
    assert_send_sync::<dyn IdSource>();
    assert_send_sync::<DefaultIdSource>();
    assert_send_sync::<Box<dyn IdSource>>();
    let boxed: Box<dyn IdSource> = Box::new(DefaultIdSource);
    let shared: Arc<dyn IdSource> = Arc::new(Counting(AtomicU64::new(0)));
    assert_ne!(boxed.next_block_id(), boxed.next_block_id());
    assert_ne!(shared.next_block_id(), shared.next_block_id());
}

#[test]
fn given_a_source_built_from_the_trait_when_asked_then_the_test_fixes_the_identities() {
    let source = Counting(AtomicU64::new(0));
    let first = source.next_block_id();
    let second = source.next_block_id();
    assert_ne!(first, second);
    let replay = Counting(AtomicU64::new(0));
    assert_eq!(replay.next_block_id(), first);
    assert_eq!(replay.next_block_id(), second);
}

#[test]
fn given_a_source_behind_a_shared_reference_when_asked_then_it_needs_no_exclusive_borrow() {
    let log = Mutex::new(Vec::new());
    let source: &dyn IdSource = &DefaultIdSource;
    log.lock().unwrap().push(source.next_block_id());
    log.lock().unwrap().push(source.next_block_id());
    assert_eq!(log.lock().unwrap().len(), 2);
}
