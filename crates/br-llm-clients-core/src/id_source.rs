use br_llm_messages::BlockId;
use uuid::Uuid;

pub trait IdSource: Send + Sync {
    fn next_block_id(&self) -> BlockId;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultIdSource;

impl IdSource for DefaultIdSource {
    fn next_block_id(&self) -> BlockId {
        loop {
            if let Ok(id) = BlockId::from_uuid(Uuid::now_v7()) {
                return id;
            }
        }
    }
}
