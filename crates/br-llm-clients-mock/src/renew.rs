use br_llm_clients_core::IdSource;
use br_llm_messages::{
    BlockId, Citation, CitationId, ContentBlock, EntryId, ImageBlock, ModelResponse, PlainBlock,
    ProviderToolCall, ProviderToolExecutionRecord, ProviderToolResult, RedactedThinkingBlock,
    RuntimeToolCall, StructuredBlock, TextBlock, ThinkingBlock,
};

use crate::derive_deltas::corrected_ending;
use crate::renew_calls::Calls;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Collision;

pub(crate) type Renewed<T> = Result<T, Collision>;

pub(crate) fn collide<T, E>(result: Result<T, E>) -> Renewed<T> {
    result.map_err(|_| Collision)
}

pub(crate) fn renew(
    source: &dyn IdSource,
    entry: EntryId,
    response: &ModelResponse,
) -> Renewed<ModelResponse> {
    let mut renewer = Renewer {
        source,
        calls: Calls::default(),
    };
    let mut blocks = Vec::with_capacity(response.blocks().len());
    for block in response.blocks() {
        blocks.push(renewer.block(block)?);
    }
    collide(ModelResponse::new(
        entry,
        blocks,
        corrected_ending(response),
        response.metadata(),
    ))
}

pub(crate) struct Renewer<'a> {
    pub(crate) source: &'a dyn IdSource,
    pub(crate) calls: Calls,
}

impl Renewer<'_> {
    fn id(&self) -> BlockId {
        self.source.next_block_id()
    }

    fn citation(&self, old: &Citation) -> Renewed<Citation> {
        let id = collide(CitationId::from_uuid(self.id().as_uuid()))?;
        let citation = Citation::new(id, old.source().clone());
        Ok(match old.range() {
            Some(range) => citation.with_range(range),
            None => citation,
        })
    }

    pub(crate) fn text(&self, old: &TextBlock) -> Renewed<TextBlock> {
        let id = self.id();
        let mut citations = Vec::with_capacity(old.citations().len());
        for citation in old.citations() {
            citations.push(self.citation(citation)?);
        }
        let mut block = collide(TextBlock::new(id, old.text().as_str()))?;
        block = collide(block.with_citations(citations))?;
        if let Some(item_ref) = old.item_ref() {
            block = block.with_item_ref(item_ref.clone());
        }
        if let Some(replay) = old.replay() {
            block = block.with_replay(replay.clone());
        }
        if let Some(observed_at) = old.observed_at() {
            block = block.with_observed_at(observed_at.clone());
        }
        Ok(block)
    }

    pub(crate) fn plain(&self, old: &PlainBlock) -> Renewed<PlainBlock> {
        Ok(match old {
            PlainBlock::Text(text) => collide(PlainBlock::try_from(self.text(text)?))?,
            PlainBlock::Image(image) => PlainBlock::Image(self.image(image)),
            PlainBlock::StructuredContent(structured) => {
                PlainBlock::StructuredContent(self.structured(structured)?)
            }
            other => other.clone(),
        })
    }

    pub(crate) fn plain_all(&self, old: &[PlainBlock]) -> Renewed<Vec<PlainBlock>> {
        let mut blocks = Vec::with_capacity(old.len());
        for block in old {
            blocks.push(self.plain(block)?);
        }
        Ok(blocks)
    }

    fn image(&self, old: &ImageBlock) -> ImageBlock {
        let block = ImageBlock::new(self.id(), old.source().clone());
        match old.observed_at() {
            Some(observed_at) => block.with_observed_at(observed_at.clone()),
            None => block,
        }
    }

    fn structured(&self, old: &StructuredBlock) -> Renewed<StructuredBlock> {
        let block = collide(StructuredBlock::new(self.id(), old.value().clone()))?;
        Ok(match old.observed_at() {
            Some(observed_at) => block.with_observed_at(observed_at.clone()),
            None => block,
        })
    }

    fn thinking(&self, old: &ThinkingBlock) -> Renewed<ThinkingBlock> {
        let id = self.id();
        let mut block = match (old.text().is_empty(), old.replay()) {
            (true, Some(replay)) => ThinkingBlock::replay_only(id, replay.clone()),
            (_, replay) => {
                let block = collide(ThinkingBlock::new(id, old.text()))?;
                match replay {
                    Some(replay) => block.with_replay(replay.clone()),
                    None => block,
                }
            }
        };
        if let Some(observed_at) = old.observed_at() {
            block = block.with_observed_at(observed_at.clone());
        }
        Ok(block)
    }

    fn redacted(&self, old: &RedactedThinkingBlock) -> RedactedThinkingBlock {
        let block = RedactedThinkingBlock::new(self.id(), old.material().clone());
        match old.observed_at() {
            Some(observed_at) => block.with_observed_at(observed_at.clone()),
            None => block,
        }
    }

    fn runtime_call(&self, old: &RuntimeToolCall) -> Renewed<RuntimeToolCall> {
        let mut call = collide(RuntimeToolCall::new(
            self.id(),
            old.name().clone(),
            old.arguments().clone(),
        ))?;
        if let Some(external_id) = old.external_id() {
            call = call.with_external_id(external_id.clone());
        }
        if let Some(item_ref) = old.item_ref() {
            call = call.with_item_ref(item_ref.clone());
        }
        if let Some(replay) = old.replay() {
            call = call.with_replay(replay.clone());
        }
        if let Some(observed_at) = old.observed_at() {
            call = call.with_observed_at(observed_at.clone());
        }
        Ok(call)
    }

    pub(crate) fn block(&mut self, old: &ContentBlock) -> Renewed<ContentBlock> {
        Ok(match old {
            ContentBlock::Text(text) => self.text(text)?.into(),
            ContentBlock::Image(image) => self.image(image).into(),
            ContentBlock::StructuredContent(structured) => self.structured(structured)?.into(),
            ContentBlock::Thinking(thinking) => self.thinking(thinking)?.into(),
            ContentBlock::RedactedThinking(redacted) => self.redacted(redacted).into(),
            ContentBlock::RuntimeToolCall(call) => self.runtime_call(call)?.into(),
            ContentBlock::ProviderToolCall(call) => self.provider_call(call)?.into(),
            ContentBlock::ProviderToolResult(result) => self.provider_result(result)?.into(),
            ContentBlock::ProviderToolExecutionRecord(record) => {
                self.execution_record(record)?.into()
            }
            other => other.clone(),
        })
    }

    fn provider_call(&mut self, old: &ProviderToolCall) -> Renewed<ProviderToolCall> {
        let id = self.id();
        self.calls.record(old.id(), id);
        let mut call = ProviderToolCall::new(id);
        if let Some(external_id) = old.external_id() {
            call = call.with_external_id(external_id.clone());
        }
        if let Some(item_ref) = old.item_ref() {
            call = call.with_item_ref(item_ref.clone());
        }
        if let Some(operation) = old.operation() {
            call = call.with_operation(operation.clone());
        }
        if let Some(arguments) = old.arguments() {
            call = collide(call.with_arguments(arguments.clone()))?;
        }
        if let Some(replay) = old.replay() {
            call = call.with_replay(replay.clone());
        }
        if let Some(observed_at) = old.observed_at() {
            call = call.with_observed_at(observed_at.clone());
        }
        Ok(call)
    }

    fn provider_result(&self, old: &ProviderToolResult) -> Renewed<ProviderToolResult> {
        let id = self.id();
        let call = self.calls.follow(old.call());
        let content = self.plain_all(old.content())?;
        let mut result = match (content.is_empty(), old.replay()) {
            (true, Some(replay)) => {
                collide(ProviderToolResult::replay_only(id, call, replay.clone()))?
            }
            (_, replay) => {
                let result = collide(ProviderToolResult::new(id, call, content))?;
                match replay {
                    Some(replay) => result.with_replay(replay.clone()),
                    None => result,
                }
            }
        };
        if let Some(external_id) = old.external_id() {
            result = result.with_external_id(external_id.clone());
        }
        if let Some(observed_at) = old.observed_at() {
            result = result.with_observed_at(observed_at.clone());
        }
        Ok(result)
    }

    fn execution_record(
        &self,
        old: &ProviderToolExecutionRecord,
    ) -> Renewed<ProviderToolExecutionRecord> {
        let id = self.id();
        let content = self.plain_all(old.content())?;
        let mut record = collide(ProviderToolExecutionRecord::new(id).with_content(content))?;
        if let Some(external_id) = old.external_id() {
            record = record.with_external_id(external_id.clone());
        }
        if let Some(operation) = old.operation() {
            record = record.with_operation(operation.clone());
        }
        if let Some(status) = old.status() {
            record = record.with_status(status);
        }
        if let Some(replay) = old.replay() {
            record = record.with_replay(replay.clone());
        }
        if let Some(observed_at) = old.observed_at() {
            record = record.with_observed_at(observed_at.clone());
        }
        Ok(record)
    }
}
