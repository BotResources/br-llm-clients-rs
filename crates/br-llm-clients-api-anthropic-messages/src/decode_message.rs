use br_llm_clients_common_call::{PayloadError, ProviderEvent};
use br_llm_messages::StopReason;
use serde_json::Value;

use crate::decode::Decoder;
use crate::decode_fields::{Fields, malformed, object_field};
use crate::decode_stop::read_stop_reason;

impl Decoder {
    pub(crate) fn message_start(
        &mut self,
        fields: &Fields,
    ) -> Result<Vec<ProviderEvent>, PayloadError> {
        if self.started {
            return Err(malformed("a second message_start"));
        }
        let message = object_field(fields, "message")?;
        self.started = true;
        let model = message
            .get("model")
            .and_then(Value::as_str)
            .filter(|model| !model.is_empty())
            .map(|model| ProviderEvent::Model(model.to_owned()));
        let mut events: Vec<ProviderEvent> = model.into_iter().collect();
        events.extend(self.merge_usage(message.get("usage"))?);
        Ok(events)
    }

    pub(crate) fn message_delta(
        &mut self,
        fields: &Fields,
    ) -> Result<Vec<ProviderEvent>, PayloadError> {
        let delta = object_field(fields, "delta")?;
        if let Some(reason) = read_stop_reason(delta)? {
            self.stop_reason = Some(reason);
        }
        Ok(self.merge_usage(fields.get("usage"))?.into_iter().collect())
    }

    pub(crate) fn message_stop(&mut self) -> Vec<ProviderEvent> {
        let mut events = if self.stop_reason == Some(StopReason::AwaitingToolResults) {
            self.blocks.release_held()
        } else {
            self.blocks.drop_held();
            Vec::new()
        };
        events.extend(self.stop_reason.clone().map(ProviderEvent::Stop));
        events.push(ProviderEvent::Complete);
        events
    }

    fn merge_usage(
        &mut self,
        usage: Option<&Value>,
    ) -> Result<Option<ProviderEvent>, PayloadError> {
        let before = self.usage;
        self.usage.merge(usage)?;
        if self.usage == before {
            return Ok(None);
        }
        Ok(self.usage.complete()?.map(ProviderEvent::Usage))
    }
}
