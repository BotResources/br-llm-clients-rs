use br_llm_messages::{
    BlockId, BlockMetadataUpdate, BlockStart, ContentBlock, DeltaOp, EntryId, Finalization,
    ObservedAt, ResponseDelta, RuntimeToolCall, StructuredBlock, TextBlock, ThinkingBlock, TurnId,
};

use crate::build_error::MockBuildError;
use crate::chunking::{Chunking, pieces};

pub(crate) struct BlockEmitter<'a> {
    turn: TurnId,
    entry: EntryId,
    chunking: &'a Chunking,
    deltas: Vec<ResponseDelta>,
}

fn failed(error: impl std::fmt::Display) -> MockBuildError {
    MockBuildError::DerivationFailed {
        reason: error.to_string(),
    }
}

fn observed(start: BlockStart, observed_at: Option<&ObservedAt>) -> BlockStart {
    match observed_at {
        Some(observed_at) => start.with_observed_at(observed_at.clone()),
        None => start,
    }
}

impl<'a> BlockEmitter<'a> {
    pub(crate) fn new(turn: TurnId, entry: EntryId, chunking: &'a Chunking) -> Self {
        Self {
            turn,
            entry,
            chunking,
            deltas: Vec::new(),
        }
    }

    pub(crate) fn push(&mut self, op: DeltaOp) {
        self.deltas
            .push(ResponseDelta::new(self.turn, self.entry, op));
    }

    pub(crate) fn into_deltas(self) -> Vec<ResponseDelta> {
        self.deltas
    }

    pub(crate) fn block(&mut self, block: &ContentBlock) -> Result<(), MockBuildError> {
        match block {
            ContentBlock::Text(text) => self.text(text),
            ContentBlock::Thinking(thinking) => {
                self.thinking(thinking);
                Ok(())
            }
            ContentBlock::RedactedThinking(redacted) => {
                let id = redacted.id();
                self.push(DeltaOp::block_started(
                    id,
                    observed(BlockStart::redacted_thinking(), redacted.observed_at()),
                ));
                self.push(DeltaOp::replay_material_set(
                    id,
                    redacted.material().clone(),
                ));
                self.finalize(id);
                Ok(())
            }
            ContentBlock::StructuredContent(structured) => self.structured(structured),
            ContentBlock::RuntimeToolCall(call) => self.tool_call(call),
            other => {
                self.push(DeltaOp::block_inserted(other.clone()));
                Ok(())
            }
        }
    }

    fn finalize(&mut self, id: BlockId) {
        self.push(DeltaOp::block_finalized(id, Finalization::accumulated()));
    }

    fn text(&mut self, block: &TextBlock) -> Result<(), MockBuildError> {
        let id = block.id();
        let mut start = BlockStart::text();
        if let Some(item_ref) = block.item_ref() {
            start = start.with_item_ref(item_ref.clone()).map_err(failed)?;
        }
        self.push(DeltaOp::block_started(
            id,
            observed(start, block.observed_at()),
        ));
        for piece in pieces(block.text().as_str(), self.chunking.text_size()) {
            self.push(DeltaOp::text_appended(id, piece));
        }
        for citation in block.citations() {
            self.push(DeltaOp::block_metadata_updated(
                id,
                BlockMetadataUpdate::citation_added(citation.clone()),
            ));
        }
        if let Some(replay) = block.replay() {
            self.push(DeltaOp::replay_material_set(id, replay.clone()));
        }
        self.finalize(id);
        Ok(())
    }

    fn thinking(&mut self, block: &ThinkingBlock) {
        let id = block.id();
        self.push(DeltaOp::block_started(
            id,
            observed(BlockStart::thinking(), block.observed_at()),
        ));
        for piece in pieces(block.text(), self.chunking.thinking_size()) {
            self.push(DeltaOp::thinking_appended(id, piece));
        }
        if let Some(replay) = block.replay() {
            self.push(DeltaOp::replay_material_set(id, replay.clone()));
        }
        self.finalize(id);
    }

    fn structured(&mut self, block: &StructuredBlock) -> Result<(), MockBuildError> {
        let id = block.id();
        let json = block.value().to_json_string().map_err(failed)?;
        self.push(DeltaOp::block_started(
            id,
            observed(BlockStart::structured_content(), block.observed_at()),
        ));
        for piece in pieces(&json, self.chunking.structured_size()) {
            self.push(DeltaOp::json_appended(id, piece));
        }
        self.finalize(id);
        Ok(())
    }

    fn tool_call(&mut self, call: &RuntimeToolCall) -> Result<(), MockBuildError> {
        let id = call.id();
        let json = call.arguments().to_json_string().map_err(failed)?;
        let mut start =
            BlockStart::runtime_tool_call(Some(call.name().clone()), call.external_id().cloned());
        if let Some(item_ref) = call.item_ref() {
            start = start.with_item_ref(item_ref.clone()).map_err(failed)?;
        }
        self.push(DeltaOp::block_started(
            id,
            observed(start, call.observed_at()),
        ));
        for piece in pieces(&json, self.chunking.tool_arguments_size()) {
            self.push(DeltaOp::json_appended(id, piece));
        }
        if let Some(replay) = call.replay() {
            self.push(DeltaOp::replay_material_set(id, replay.clone()));
        }
        self.finalize(id);
        Ok(())
    }
}
