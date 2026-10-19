use br_llm_clients_common_call::{PayloadError, ProviderBlock, ProviderEvent};
use serde_json::Value;

use super::event::EventType;
use super::items::{CallItem, DoneCall, ItemState, Items};
use super::{ResponsesDecoder, open_slot};

fn non_empty(text: Option<&str>) -> Option<String> {
    text.filter(|text| !text.is_empty()).map(str::to_owned)
}

impl ResponsesDecoder {
    pub(super) fn arguments_delta(
        &mut self,
        data: &Value,
    ) -> Result<Vec<ProviderEvent>, PayloadError> {
        let event = EventType::ArgumentsDelta;
        let output_index = event.index(data, "output_index")?;
        let delta = event.text(data, "delta")?;
        let slot = open_slot(&mut self.items, event, output_index)?;
        let ItemState::Call(call) = &mut slot.state else {
            return Err(event.fail("the delta does not fit the item"));
        };
        call.deltas.push_str(delta);
        Ok(Vec::new())
    }
}

pub(super) fn done(call: &mut CallItem, item: &Value) -> Result<Vec<ProviderEvent>, PayloadError> {
    let event = EventType::ItemDone;
    call.done = Some(DoneCall {
        name: non_empty(event.optional_text(item, "name")?),
        call_id: non_empty(event.optional_text(item, "call_id")?),
        arguments: event.text(item, "arguments")?.to_owned(),
        completed: item.get("status").and_then(Value::as_str) == Some("completed"),
    });
    Ok(Vec::new())
}

pub(super) fn held_events(
    event: EventType,
    items: &Items,
) -> Result<Vec<ProviderEvent>, PayloadError> {
    let mut events = Vec::new();
    for (output_index, slot) in items {
        let ItemState::Call(call) = &slot.state else {
            continue;
        };
        events.extend(events_of(event, *output_index, call)?);
    }
    Ok(events)
}

fn events_of(
    event: EventType,
    output_index: u64,
    call: &CallItem,
) -> Result<Vec<ProviderEvent>, PayloadError> {
    let done = call.done.as_ref();
    let name = done
        .and_then(|done| done.name.clone())
        .or_else(|| non_empty(call.name.as_deref()))
        .ok_or_else(|| event.fail("a function call has no name"))?;
    let id = done
        .and_then(|done| done.call_id.clone())
        .or_else(|| non_empty(call.call_id.as_deref()));
    let completed = done.is_some_and(|done| done.completed);
    let arguments = done.map_or(call.deltas.as_str(), |done| done.arguments.as_str());
    let arguments = if completed && arguments.is_empty() {
        "{}"
    } else {
        arguments
    };
    let key = output_index.to_string();
    let mut events = vec![
        ProviderEvent::BlockStart {
            key: key.clone(),
            block: ProviderBlock::ToolCall { name, id },
        },
        ProviderEvent::ToolArgumentsDelta {
            key: key.clone(),
            json: arguments.to_owned(),
        },
    ];
    if completed {
        events.push(ProviderEvent::BlockEnd { key });
    }
    Ok(events)
}
