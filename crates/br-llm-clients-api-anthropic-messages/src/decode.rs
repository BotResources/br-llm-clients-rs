use br_llm_clients_common_call::{End, Payload, PayloadDecoder, PayloadError, ProviderEvent};
use br_llm_messages::ResponseEnding;

use crate::decode_blocks::Blocks;
use crate::decode_envelope::{Envelope, read};
use crate::decode_fields::{Fields, malformed};
use crate::decode_usage::UsageRecord;

pub(crate) struct Decoder {
    pub(crate) blocks: Blocks,
    pub(crate) started: bool,
    pub(crate) stop_reason: Option<ResponseEnding>,
    pub(crate) usage: UsageRecord,
}

impl Decoder {
    pub(crate) fn new(structured: bool) -> Self {
        Self {
            blocks: Blocks::new(structured),
            started: false,
            stop_reason: None,
            usage: UsageRecord::default(),
        }
    }

    fn dispatch(
        &mut self,
        kind: &str,
        fields: &Fields,
    ) -> Result<Vec<ProviderEvent>, PayloadError> {
        match kind {
            "ping" => Ok(Vec::new()),
            "message_start" => self.message_start(fields),
            "content_block_start"
            | "content_block_delta"
            | "content_block_stop"
            | "message_delta"
            | "message_stop" => {
                if !self.started {
                    return Err(malformed("an event arrived before message_start"));
                }
                self.in_message(kind, fields)
            }
            _ => Ok(Vec::new()),
        }
    }

    fn in_message(
        &mut self,
        kind: &str,
        fields: &Fields,
    ) -> Result<Vec<ProviderEvent>, PayloadError> {
        match kind {
            "content_block_start" => self.blocks.start(fields),
            "content_block_delta" => self.blocks.delta(fields),
            "content_block_stop" => self.blocks.stop(fields),
            "message_delta" => self.message_delta(fields),
            _ => Ok(self.message_stop()),
        }
    }
}

impl PayloadDecoder for Decoder {
    fn payload(&mut self, payload: Payload) -> Result<Vec<ProviderEvent>, PayloadError> {
        match read(&payload)? {
            Envelope::Error(info) => Ok(vec![ProviderEvent::Error(info)]),
            Envelope::Event { kind, fields } => self.dispatch(&kind, &fields),
        }
    }

    fn end(&mut self) -> Result<End, PayloadError> {
        Ok(End::MarkerMissing)
    }
}
