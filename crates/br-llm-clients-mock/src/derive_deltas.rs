use br_llm_messages::{
    DeltaOp, ModelResponse, ResponseDelta, ResponseEnding, ResponseMetadata,
    ResponseMetadataUpdate, TurnId,
};

use crate::block_deltas::BlockEmitter;
use crate::build_error::MockBuildError;
use crate::chunking::Chunking;

#[derive(Debug, Clone, PartialEq)]
pub struct DerivedDeltas {
    deltas: Vec<ResponseDelta>,
}

impl DerivedDeltas {
    pub fn deltas(&self) -> &[ResponseDelta] {
        &self.deltas
    }

    pub fn before_completed(&self) -> usize {
        self.deltas.len().saturating_sub(1)
    }

    pub fn into_deltas(self) -> Vec<ResponseDelta> {
        self.deltas
    }
}

pub(crate) fn corrected_ending(response: &ModelResponse) -> ResponseEnding {
    if response.runtime_calls().next().is_some() {
        ResponseEnding::tool_use()
    } else {
        response.ending().clone()
    }
}

fn started(response: &ModelResponse) -> ResponseMetadata {
    let mut metadata = ResponseMetadata::new();
    if let Some(model) = response.model() {
        metadata = metadata.with_model(model.clone());
    }
    if let Some(id) = response.provider_response_id() {
        metadata = metadata.with_provider_response_id(id.clone());
    }
    if let Some(observed_at) = response.observed_at() {
        metadata = metadata.with_observed_at(observed_at.clone());
    }
    metadata
}

pub fn derive_deltas(
    turn: TurnId,
    response: &ModelResponse,
    chunking: &Chunking,
) -> Result<DerivedDeltas, MockBuildError> {
    chunking.check()?;
    let entry = response.id();
    let mut emitter = BlockEmitter::new(turn, entry, chunking);
    emitter.push(DeltaOp::response_started(started(response)));
    for block in response.blocks() {
        emitter.block(block)?;
    }
    if let Some(usage) = response.usage() {
        emitter.push(DeltaOp::response_metadata_updated(
            ResponseMetadataUpdate::new().with_usage(usage.clone()),
        ));
    }
    emitter.push(DeltaOp::response_completed(corrected_ending(response)));
    Ok(DerivedDeltas {
        deltas: emitter.into_deltas(),
    })
}
