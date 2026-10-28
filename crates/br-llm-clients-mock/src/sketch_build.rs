use br_llm_messages::{
    BlockId, ContentBlock, EntryId, Error, ExternalId, ModelName, ModelResponse, ResponseEnding,
    ResponseMetadata, RuntimeToolCall, StructuredBlock, TextBlock, ThinkingBlock, ToolName,
};

use crate::sequential_id_source::identity_uuid;
use crate::sketch::{Item, Sketch};

pub(crate) fn placeholder_block(number: u64) -> Result<BlockId, Error> {
    BlockId::from_uuid(identity_uuid(number))
}

pub(crate) fn placeholder_entry() -> Result<EntryId, Error> {
    EntryId::from_uuid(identity_uuid(0))
}

pub(crate) fn build(sketch: &Sketch) -> Result<ModelResponse, Error> {
    let mut blocks = Vec::with_capacity(sketch.items.len());
    for (index, item) in sketch.items.iter().enumerate() {
        let number = u64::try_from(index).unwrap_or(u64::MAX).saturating_add(1);
        blocks.push(block(placeholder_block(number)?, item)?);
    }
    let has_call = blocks
        .iter()
        .any(|block| matches!(block, ContentBlock::RuntimeToolCall(_)));
    let ending = match (&sketch.ending, has_call) {
        (Some(ending), _) => ending.clone(),
        (None, true) => ResponseEnding::tool_use(),
        (None, false) => ResponseEnding::finished(),
    };
    ModelResponse::new(placeholder_entry()?, blocks, ending, metadata(sketch)?)
}

fn block(id: BlockId, item: &Item) -> Result<ContentBlock, Error> {
    Ok(match item {
        Item::Text(text) => TextBlock::new(id, text.clone())?.into(),
        Item::Thinking { text, replay } => match (text.is_empty(), replay) {
            (true, Some(replay)) => ThinkingBlock::replay_only(id, replay.clone()).into(),
            (_, Some(replay)) => ThinkingBlock::new(id, text.clone())?
                .with_replay(replay.clone())
                .into(),
            (_, None) => ThinkingBlock::new(id, text.clone())?.into(),
        },
        Item::ToolCall {
            name,
            external_id,
            arguments,
        } => {
            let call = RuntimeToolCall::new(id, ToolName::new(name.clone())?, arguments.parsed()?)?;
            match external_id {
                Some(external_id) => call
                    .with_external_id(ExternalId::new(external_id.clone())?)
                    .into(),
                None => call.into(),
            }
        }
        Item::Structured(value) => StructuredBlock::new(id, value.parsed()?)?.into(),
    })
}

fn metadata(sketch: &Sketch) -> Result<ResponseMetadata, Error> {
    let mut metadata = ResponseMetadata::new();
    if let Some(usage) = &sketch.usage {
        metadata = metadata.with_usage(usage.clone());
    }
    if let Some(model) = &sketch.model {
        metadata = metadata.with_model(ModelName::new(model.clone())?);
    }
    if let Some(id) = &sketch.provider_response_id {
        metadata = metadata.with_provider_response_id(ExternalId::new(id.clone())?);
    }
    Ok(metadata)
}
