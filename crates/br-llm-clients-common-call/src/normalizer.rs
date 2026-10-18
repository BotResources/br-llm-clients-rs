use br_llm_clients_core::{ClientError, ClientRequest};
use br_llm_messages::{ModelId, StopReason, StreamEvent, ToolName, Usage};

use crate::block::{Block, Kind, Raw};
use crate::failure_map::malformed;
use crate::generated_id::ToolIds;
use crate::judge::Held;
use crate::provider_event::{ProviderBlock, ProviderEvent};

pub(crate) struct Normalizer {
    blocks: Vec<Block>,
    head: usize,
    next_index: usize,
    ids: ToolIds,
    stop: Option<StopReason>,
    usage: Option<Usage>,
    model: Option<ModelId>,
}

impl Normalizer {
    pub(crate) fn new(request: &ClientRequest) -> Self {
        Self {
            blocks: Vec::new(),
            head: 0,
            next_index: 0,
            ids: ToolIds::of_request(request),
            stop: None,
            usage: None,
            model: None,
        }
    }

    pub(crate) fn stop(&self) -> Option<&StopReason> {
        self.stop.as_ref()
    }

    pub(crate) fn usage(&self) -> Option<&Usage> {
        self.usage.as_ref()
    }

    pub(crate) fn model(&self) -> Option<&ModelId> {
        self.model.as_ref()
    }

    pub(crate) fn held(&self) -> Vec<Held> {
        self.blocks.iter().map(Block::held).collect()
    }

    pub(crate) fn accept(&mut self, event: ProviderEvent) -> Result<Vec<StreamEvent>, ClientError> {
        match event {
            ProviderEvent::Stop(reason) => self.set_stop(reason),
            ProviderEvent::Usage(usage) => {
                self.usage = Some(usage);
                Ok(Vec::new())
            }
            ProviderEvent::Model(text) => self.set_model(&text),
            ProviderEvent::Error(_) | ProviderEvent::Complete => Ok(Vec::new()),
            ProviderEvent::BlockStart { key, block } => self.start(key, block),
            ProviderEvent::BlockEnd { key } => self.route(&key, Raw::End),
            ProviderEvent::TextDelta { key, text } => self.route(&key, Raw::Text(text)),
            ProviderEvent::ThinkingDelta { key, text } => self.route(&key, Raw::Thinking(text)),
            ProviderEvent::SignatureDelta { key, signature } => {
                self.route(&key, Raw::Signature(signature))
            }
            ProviderEvent::StructuredDelta { key, json } => self.route(&key, Raw::Structured(json)),
            ProviderEvent::ToolArgumentsDelta { key, json } => {
                self.route(&key, Raw::Arguments(json))
            }
            ProviderEvent::RedactedThinkingData { key, data } => {
                self.route(&key, Raw::Redacted(data))
            }
        }
    }

    pub(crate) fn close_all(&mut self) -> Vec<StreamEvent> {
        for block in self.blocks.iter_mut().filter(|block| !block.end_queued) {
            block.push_forced_end();
        }
        self.pump()
    }

    fn set_stop(&mut self, reason: StopReason) -> Result<Vec<StreamEvent>, ClientError> {
        if self.stop.is_some() {
            return Err(malformed("the provider gave a second stop reason"));
        }
        self.stop = Some(reason);
        Ok(Vec::new())
    }

    fn set_model(&mut self, text: &str) -> Result<Vec<StreamEvent>, ClientError> {
        let model = ModelId::new(text).map_err(|_| malformed("the model is not valid"))?;
        self.model = Some(model);
        Ok(Vec::new())
    }

    fn open_position(&self, key: &str) -> Option<usize> {
        self.blocks
            .iter()
            .rposition(|block| block.key == key && !block.end_queued)
    }

    fn start(
        &mut self,
        key: String,
        block: ProviderBlock,
    ) -> Result<Vec<StreamEvent>, ClientError> {
        self.check_not_stopped()?;
        if self.open_position(&key).is_some() {
            return Err(malformed("a block was started for a key that is open"));
        }
        let kind = self.kind_of(block)?;
        self.blocks.push(Block::new(key, kind));
        Ok(self.pump())
    }

    fn route(&mut self, key: &str, raw: Raw) -> Result<Vec<StreamEvent>, ClientError> {
        self.check_not_stopped()?;
        let position = self
            .open_position(key)
            .ok_or_else(|| malformed("an event came for a key that is not open"))?;
        let block = self
            .blocks
            .get_mut(position)
            .ok_or_else(|| malformed("an event came for a key that is not open"))?;
        if !matches!(raw, Raw::End) && !block.accepts(&raw) {
            return Err(malformed("a delta does not fit the block of its key"));
        }
        block.push(raw);
        Ok(self.pump())
    }

    fn check_not_stopped(&self) -> Result<(), ClientError> {
        if self.stop.is_some() {
            return Err(malformed("a block event came after the stop reason"));
        }
        Ok(())
    }

    fn kind_of(&mut self, block: ProviderBlock) -> Result<Kind, ClientError> {
        Ok(match block {
            ProviderBlock::Text => Kind::Text,
            ProviderBlock::Thinking => Kind::Thinking,
            ProviderBlock::RedactedThinking => Kind::Redacted,
            ProviderBlock::Structured => Kind::Structured,
            ProviderBlock::ToolCall { name, id } => {
                let name = ToolName::new(name)
                    .map_err(|_| malformed("the name of a tool call is not valid"))?;
                let id = match id.filter(|id| !id.is_empty()) {
                    Some(id) => self.ids.claim(&id)?,
                    None => self.ids.generate()?,
                };
                Kind::ToolCall { id, name }
            }
        })
    }

    fn pump(&mut self) -> Vec<StreamEvent> {
        let mut out = Vec::new();
        while let Some(block) = self.blocks.get_mut(self.head) {
            block.drain(&mut self.next_index, &mut out);
            if !block.is_finished() {
                break;
            }
            self.head = self.head.saturating_add(1);
        }
        out
    }
}
