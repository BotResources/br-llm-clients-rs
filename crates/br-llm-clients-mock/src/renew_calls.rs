use std::collections::HashMap;

use br_llm_messages::BlockId;

#[derive(Debug, Default)]
pub(crate) struct Calls {
    renamed: HashMap<BlockId, BlockId>,
}

impl Calls {
    pub(crate) fn record(&mut self, old: BlockId, new: BlockId) {
        self.renamed.insert(old, new);
    }

    pub(crate) fn follow(&self, named: BlockId) -> BlockId {
        self.renamed.get(&named).copied().unwrap_or(named)
    }
}
