use br_llm_clients_common_call::{ErrorInfo, PayloadError, ProviderEvent};
use br_llm_messages::ResponseEnding;
use serde_json::Value;

use super::event::EventType;
use super::items::ItemState;
use super::usage::usage_of;
use super::{ResponsesDecoder, call, message, reasoning};
use crate::error_envelope::error_envelope;

const NO_ERROR_TEXT: &str = "response.failed holds no error";
const NO_REASON_TEXT: &str = "incomplete";

fn model_event(event: EventType, response: &Value) -> Result<Option<ProviderEvent>, PayloadError> {
    Ok(event
        .optional_text(response, "model")?
        .filter(|model| !model.is_empty())
        .map(|model| ProviderEvent::Model(model.to_owned())))
}

pub(super) fn created(data: &Value) -> Result<Vec<ProviderEvent>, PayloadError> {
    let event = EventType::Created;
    let response = event.object(data, "response")?;
    let mut events: Vec<ProviderEvent> = model_event(event, response)?.into_iter().collect();
    if let Some(id) = event
        .optional_text(response, "id")?
        .filter(|id| !id.is_empty())
    {
        events.push(ProviderEvent::ResponseId(id.to_owned()));
    }
    Ok(events)
}

fn as_text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn info_of(code: Option<&str>, message: Option<&str>, param: Option<&str>) -> ErrorInfo {
    let info = ErrorInfo::new(error_envelope(message, param, code));
    match code {
        Some(code) => info.with_code(code),
        None => info,
    }
}

fn error_info(holder: &Value) -> ErrorInfo {
    info_of(
        as_text(holder, "code"),
        as_text(holder, "message"),
        as_text(holder, "param"),
    )
}

fn event_member<'a>(data: &'a Value, nested: Option<&'a Value>, key: &str) -> Option<&'a str> {
    as_text(data, key).or_else(|| nested.and_then(|nested| as_text(nested, key)))
}

pub(super) fn error_event(data: &Value) -> Vec<ProviderEvent> {
    let nested = data.get("error").filter(|error| error.is_object());
    vec![ProviderEvent::Error(info_of(
        event_member(data, nested, "code"),
        event_member(data, nested, "message"),
        event_member(data, nested, "param"),
    ))]
}

pub(super) fn failed(data: &Value) -> Result<Vec<ProviderEvent>, PayloadError> {
    let event = EventType::Failed;
    let response = event.object(data, "response")?;
    let usage = usage_of(event, response)?;
    let model = model_event(event, response)?;
    let info = match response.get("error").filter(|error| error.is_object()) {
        Some(error) => error_info(error),
        None => ErrorInfo::new(NO_ERROR_TEXT),
    };
    let mut events = Vec::new();
    events.extend(usage.map(ProviderEvent::Usage));
    events.extend(model);
    events.push(ProviderEvent::Error(info));
    Ok(events)
}

fn incomplete_ending(event: EventType, response: &Value) -> Result<ResponseEnding, PayloadError> {
    let details = match response.get("incomplete_details") {
        None | Some(Value::Null) => None,
        Some(found) if found.is_object() => Some(found),
        Some(_) => return Err(event.fail("incomplete_details is not an object")),
    };
    let reason = match details {
        Some(details) => event.optional_text(details, "reason")?,
        None => None,
    };
    match reason {
        Some("max_output_tokens") => Ok(ResponseEnding::output_exhaustion()),
        Some("content_filter") => Ok(ResponseEnding::filtering()),
        Some("") => Err(event.fail("incomplete_details.reason is an empty text")),
        Some(other) => ResponseEnding::unclassified(other)
            .map_err(|_| event.fail("incomplete_details.reason cannot be an ending")),
        None => ResponseEnding::unclassified(NO_REASON_TEXT)
            .map_err(|_| event.fail("the ending of an incomplete response cannot be built")),
    }
}

impl ResponsesDecoder {
    pub(super) fn finish(
        &mut self,
        event: EventType,
        data: &Value,
    ) -> Result<Vec<ProviderEvent>, PayloadError> {
        let response = event.object(data, "response")?;
        let usage = usage_of(event, response)?;
        let model = model_event(event, response)?;
        let stop = if event == EventType::Incomplete {
            incomplete_ending(event, response)?
        } else if self.refusal_seen {
            ResponseEnding::refusal()
        } else {
            ResponseEnding::finished()
        };
        let mut events = self.close_undone();
        events.extend(call::held_events(event, &self.items)?);
        events.extend(usage.map(ProviderEvent::Usage));
        events.extend(model);
        events.push(ProviderEvent::Stop(stop));
        events.push(ProviderEvent::Complete);
        Ok(events)
    }

    fn close_undone(&self) -> Vec<ProviderEvent> {
        let mut events = Vec::new();
        for (output_index, slot) in self.items.iter().filter(|(_, slot)| !slot.done) {
            match &slot.state {
                ItemState::Message(item) => {
                    events.extend(message::finish_undone(item, *output_index, self.structured));
                }
                ItemState::Reasoning(item) => {
                    events.extend(reasoning::finish_undone(item, *output_index));
                }
                ItemState::Call(_) => {}
            }
        }
        events
    }
}
