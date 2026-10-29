use std::sync::atomic::{AtomicU64, Ordering};

use br_llm_clients_core::IdSource;
use br_llm_messages::{BlockId, EntryId, TurnId, Uuid};

pub(crate) fn uuid_of(n: u64) -> Uuid {
    Uuid::from_u128(0x0190_0000_0000_7000_8000_0000_0000_0000 | u128::from(n))
}

pub(crate) fn number_of(id: BlockId) -> u64 {
    u64::try_from(id.as_uuid().as_u128() & 0xFFFF_FFFF).unwrap_or(u64::MAX)
}

pub(crate) fn block_of(n: u64) -> BlockId {
    BlockId::from_uuid(uuid_of(n)).expect("a block id")
}

pub(crate) fn reply_turn() -> TurnId {
    TurnId::from_uuid(uuid_of(0x1000)).expect("a turn id")
}

pub(crate) fn reply_entry() -> EntryId {
    EntryId::from_uuid(uuid_of(0x2000)).expect("an entry id")
}

pub(crate) struct CountingIds {
    next: AtomicU64,
    taken: AtomicU64,
}

impl CountingIds {
    pub(crate) fn from(first: u64) -> Self {
        Self {
            next: AtomicU64::new(first),
            taken: AtomicU64::new(0),
        }
    }

    pub(crate) fn new() -> Self {
        Self::from(1)
    }

    pub(crate) fn taken(&self) -> u64 {
        self.taken.load(Ordering::SeqCst)
    }
}

impl IdSource for CountingIds {
    fn next_block_id(&self) -> BlockId {
        self.taken.fetch_add(1, Ordering::SeqCst);
        block_of(self.next.fetch_add(1, Ordering::SeqCst))
    }
}
