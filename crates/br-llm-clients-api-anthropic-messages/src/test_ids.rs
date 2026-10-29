use std::sync::atomic::{AtomicU64, Ordering};

use br_llm_clients_core::IdSource;
use br_llm_messages::BlockId;

use crate::test_thread::uuid_of;

pub(crate) struct CountingIds {
    next: AtomicU64,
}

impl CountingIds {
    pub(crate) fn new() -> Self {
        Self {
            next: AtomicU64::new(0x8000_0000),
        }
    }
}

impl IdSource for CountingIds {
    fn next_block_id(&self) -> BlockId {
        BlockId::from_uuid(uuid_of(self.next.fetch_add(1, Ordering::SeqCst))).expect("a block id")
    }
}
