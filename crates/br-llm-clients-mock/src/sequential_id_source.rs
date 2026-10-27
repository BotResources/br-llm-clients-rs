use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use br_llm_clients_core::IdSource;
use br_llm_messages::{BlockId, EntryId, TurnId, Uuid};

pub const DEFAULT_ID_START: u64 = 0x2000_0000_0000_0000;

const LAST_NUMBER: u64 = 0x3FFF_FFFF_FFFF_FFFF;
const VERSION_7_BITS: u128 = 0x7 << 76;
const RFC_4122_VARIANT_BITS: u128 = 0x8000_0000_0000_0000;

pub(crate) fn identity_uuid(number: u64) -> Uuid {
    Uuid::from_u128(u128::from(number.saturating_add(1)) | VERSION_7_BITS | RFC_4122_VARIANT_BITS)
}

#[derive(Debug, Clone)]
pub struct SequentialIdSource {
    start: u64,
    given: Arc<AtomicU64>,
}

impl Default for SequentialIdSource {
    fn default() -> Self {
        Self::new()
    }
}

impl SequentialIdSource {
    pub fn new() -> Self {
        Self::starting_at(0)
    }

    pub fn starting_at(start: u64) -> Self {
        Self {
            start,
            given: Arc::new(AtomicU64::new(0)),
        }
    }

    pub(crate) fn of_mock() -> Self {
        Self::starting_at(DEFAULT_ID_START)
    }

    pub fn next_number(&self) -> u64 {
        self.start.saturating_add(self.given.load(Ordering::SeqCst))
    }

    pub fn given(&self) -> u64 {
        self.given.load(Ordering::SeqCst)
    }

    pub fn next_entry_id(&self) -> EntryId {
        loop {
            if let Ok(id) = EntryId::from_uuid(self.take()) {
                return id;
            }
        }
    }

    pub fn next_turn_id(&self) -> TurnId {
        loop {
            if let Ok(id) = TurnId::from_uuid(self.take()) {
                return id;
            }
        }
    }

    fn take(&self) -> Uuid {
        let start = self.start;
        let taken = self
            .given
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |given| {
                let number = start.checked_add(given)?;
                (number < LAST_NUMBER).then(|| given.saturating_add(1))
            });
        match taken {
            Ok(given) => identity_uuid(start.saturating_add(given)),
            Err(_) => panic!("the sequential source of identities is exhausted"),
        }
    }
}

impl IdSource for SequentialIdSource {
    fn next_block_id(&self) -> BlockId {
        loop {
            if let Ok(id) = BlockId::from_uuid(self.take()) {
                return id;
            }
        }
    }
}
