use std::sync::Arc;

use br_llm_clients_core::{ClientError, ClientRequest, DefaultIdSource, IdSource};
use br_llm_messages::{
    DeltaOp, EntryId, ExternalId, ModelName, ResponseDelta, ResponseEnding, ResponseMetadata,
    ResponseMetadataUpdate, ToolName, TurnId, Usage,
};

use crate::block::{Block, Emit, Kind, Op};
use crate::failure_map::malformed;
use crate::judge::Held;
use crate::provider_event::{ProviderBlock, ProviderEvent};

#[derive(Clone)]
pub(crate) struct Normalizer {
    blocks: Vec<Block>,
    head: usize,
    ids: Arc<dyn IdSource>,
    turn: TurnId,
    entry: EntryId,
    answered: Option<ModelName>,
    started: bool,
    stop: Option<ResponseEnding>,
    usage: Option<Usage>,
    model: Option<ModelName>,
}

impl Normalizer {
    pub(crate) fn new(
        request: &ClientRequest,
        ids: Arc<dyn IdSource>,
        answered: Option<ModelName>,
    ) -> Self {
        Self {
            blocks: Vec::new(),
            head: 0,
            ids,
            turn: request.reply_turn(),
            entry: request.reply_entry(),
            answered,
            started: false,
            stop: None,
            usage: None,
            model: None,
        }
    }

    pub(crate) fn rehearsal(&self) -> Self {
        Self {
            ids: Arc::new(DefaultIdSource),
            ..self.clone()
        }
    }

    pub(crate) fn stop(&self) -> Option<&ResponseEnding> {
        self.stop.as_ref()
    }

    pub(crate) fn usage(&self) -> Option<&Usage> {
        self.usage.as_ref()
    }

    pub(crate) fn model(&self) -> Option<&ModelName> {
        self.model.as_ref()
    }

    pub(crate) fn held(&self) -> Vec<Held> {
        self.blocks.iter().map(Block::held).collect()
    }

    pub(crate) fn completed(&self, ending: ResponseEnding) -> ResponseDelta {
        self.delta(DeltaOp::response_completed(ending))
    }

    pub(crate) fn accept(
        &mut self,
        event: ProviderEvent,
    ) -> Result<Vec<ResponseDelta>, ClientError> {
        let mut out = Vec::new();
        match event {
            ProviderEvent::Error(_) | ProviderEvent::Complete => {}
            ProviderEvent::Stop(ending) => self.set_stop(ending, &mut out)?,
            ProviderEvent::Usage(usage) => {
                self.begin(&mut out);
                self.usage = Some(usage.clone());
                out.push(self.update(ResponseMetadataUpdate::new().with_usage(usage)));
            }
            ProviderEvent::Model(text) => self.set_model(&text, &mut out)?,
            ProviderEvent::ResponseId(text) => self.set_response_id(text, &mut out),
            ProviderEvent::BlockStart { key, block } => self.start(key, block, &mut out)?,
            ProviderEvent::BlockEnd { key } => self.route(&key, Op::End, &mut out)?,
            ProviderEvent::TextDelta { key, text } => self.route(&key, Op::Text(text), &mut out)?,
            ProviderEvent::ThinkingDelta { key, text } => {
                self.route(&key, Op::Thinking(text), &mut out)?;
            }
            ProviderEvent::SignatureDelta {
                key,
                format,
                signature,
            } => self.route(
                &key,
                Op::Replay {
                    format,
                    text: signature,
                },
                &mut out,
            )?,
            ProviderEvent::StructuredDelta { key, json }
            | ProviderEvent::ToolArgumentsDelta { key, json } => {
                self.route(&key, Op::Json(json), &mut out)?;
            }
            ProviderEvent::RedactedThinkingData { key, format, data } => {
                self.route(&key, Op::Replay { format, text: data }, &mut out)?;
            }
        }
        Ok(out)
    }

    pub(crate) fn close_all(&mut self) -> Vec<ResponseDelta> {
        for block in self.blocks.iter_mut().filter(|block| !block.closed) {
            block.push_forced_end();
        }
        let mut out = Vec::new();
        self.pump(&mut out);
        out
    }

    fn delta(&self, op: DeltaOp) -> ResponseDelta {
        ResponseDelta::new(self.turn, self.entry, op)
    }

    fn update(&self, update: ResponseMetadataUpdate) -> ResponseDelta {
        self.delta(DeltaOp::response_metadata_updated(update))
    }

    fn begin(&mut self, out: &mut Vec<ResponseDelta>) {
        if self.started {
            return;
        }
        self.started = true;
        let metadata = match &self.answered {
            Some(model) => ResponseMetadata::new().with_model(model.clone()),
            None => ResponseMetadata::new(),
        };
        out.push(self.delta(DeltaOp::response_started(metadata)));
    }

    fn set_stop(
        &mut self,
        ending: ResponseEnding,
        out: &mut Vec<ResponseDelta>,
    ) -> Result<(), ClientError> {
        if self.stop.is_some() {
            return Err(malformed("the provider gave a second ending"));
        }
        self.begin(out);
        self.stop = Some(ending);
        Ok(())
    }

    fn set_model(&mut self, text: &str, out: &mut Vec<ResponseDelta>) -> Result<(), ClientError> {
        let model = ModelName::new(text).map_err(|_| malformed("the model is not valid"))?;
        self.begin(out);
        self.model = Some(model.clone());
        out.push(self.update(ResponseMetadataUpdate::new().with_model(model)));
        Ok(())
    }

    fn set_response_id(&mut self, text: String, out: &mut Vec<ResponseDelta>) {
        self.begin(out);
        if let Ok(id) = ExternalId::new(text) {
            out.push(self.update(ResponseMetadataUpdate::new().with_provider_response_id(id)));
        }
    }

    fn open_position(&self, key: &str) -> Option<usize> {
        self.blocks
            .iter()
            .rposition(|block| block.key == key && !block.closed)
    }

    fn start(
        &mut self,
        key: String,
        block: ProviderBlock,
        out: &mut Vec<ResponseDelta>,
    ) -> Result<(), ClientError> {
        self.check_not_stopped()?;
        if self.open_position(&key).is_some() {
            return Err(malformed("a block was started for a key that is open"));
        }
        let kind = kind_of(block)?;
        self.begin(out);
        self.blocks.push(Block::new(key, kind));
        self.pump(out);
        Ok(())
    }

    fn route(
        &mut self,
        key: &str,
        op: Op,
        out: &mut Vec<ResponseDelta>,
    ) -> Result<(), ClientError> {
        self.check_not_stopped()?;
        let position = self
            .open_position(key)
            .ok_or_else(|| malformed("an event came for a key that is not open"))?;
        let fits = self
            .blocks
            .get(position)
            .is_some_and(|block| matches!(op, Op::End) || block.accepts(&op));
        if !fits {
            return Err(malformed("a delta does not fit the block of its key"));
        }
        self.begin(out);
        if let Some(block) = self.blocks.get_mut(position) {
            block.push(op);
        }
        self.pump(out);
        Ok(())
    }

    fn check_not_stopped(&self) -> Result<(), ClientError> {
        if self.stop.is_some() {
            return Err(malformed("a block event came after the ending"));
        }
        Ok(())
    }

    fn pump(&mut self, out: &mut Vec<ResponseDelta>) {
        let mut cx = Emit {
            ids: &*self.ids,
            turn: self.turn,
            entry: self.entry,
            out,
        };
        while let Some(block) = self.blocks.get_mut(self.head) {
            block.drain(&mut cx);
            if !block.is_finished() {
                break;
            }
            self.head = self.head.saturating_add(1);
        }
    }
}

fn kind_of(block: ProviderBlock) -> Result<Kind, ClientError> {
    Ok(match block {
        ProviderBlock::Text => Kind::Text,
        ProviderBlock::Thinking => Kind::Thinking,
        ProviderBlock::RedactedThinking => Kind::Redacted,
        ProviderBlock::Structured => Kind::Structured,
        ProviderBlock::ToolCall { name, id } => {
            let name = ToolName::new(name)
                .map_err(|_| malformed("the name of a tool call is not valid"))?;
            let external_id = id.and_then(|id| ExternalId::new(id).ok());
            Kind::ToolCall { name, external_id }
        }
    })
}
