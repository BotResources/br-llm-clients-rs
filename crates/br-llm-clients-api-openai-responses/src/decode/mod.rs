mod call;
mod event;
mod items;
mod message;
mod reasoning;
mod terminal;
mod usage;

use br_llm_clients_common_call::{End, Payload, PayloadDecoder, PayloadError, ProviderEvent};
use serde_json::Value;

use self::event::EventType;
use self::items::{CallItem, ItemState, Items, MessageItem, ReasoningItem, Slot};

pub(crate) struct ResponsesDecoder {
    structured: bool,
    items: Items,
    refusal_seen: bool,
}

impl ResponsesDecoder {
    pub(crate) fn new(structured: bool) -> Self {
        Self {
            structured,
            items: Items::new(),
            refusal_seen: false,
        }
    }

    fn handle(
        &mut self,
        event: EventType,
        data: &Value,
    ) -> Result<Vec<ProviderEvent>, PayloadError> {
        match event {
            EventType::Created => terminal::created(data),
            EventType::ItemAdded => self.item_added(data),
            EventType::PartAdded => self.part_added(data),
            EventType::TextDelta | EventType::RefusalDelta => self.text_delta(event, data),
            EventType::SummaryDelta | EventType::ReasoningDelta => self.thinking_delta(event, data),
            EventType::ArgumentsDelta => self.arguments_delta(data),
            EventType::ItemDone => self.item_done(data),
            EventType::Error => Ok(terminal::error_event(data)),
            EventType::Completed | EventType::Incomplete => self.finish(event, data),
            EventType::Failed => terminal::failed(data),
        }
    }

    fn item_added(&mut self, data: &Value) -> Result<Vec<ProviderEvent>, PayloadError> {
        let event = EventType::ItemAdded;
        let index = event.index(data, "output_index")?;
        let item = event.object(data, "item")?;
        let state = match event.text(item, "type")? {
            "message" if item.get("role").and_then(Value::as_str) == Some("assistant") => {
                ItemState::Message(MessageItem::default())
            }
            "reasoning" => ItemState::Reasoning(ReasoningItem::default()),
            "function_call" => ItemState::Call(CallItem {
                name: event.optional_text(item, "name")?.map(str::to_owned),
                call_id: event.optional_text(item, "call_id")?.map(str::to_owned),
                deltas: String::new(),
                done: None,
            }),
            _ => return Err(event.fail("the item is of a kind that is not known")),
        };
        if self.items.contains_key(&index) {
            return Err(event.fail("the item was added twice"));
        }
        self.items.insert(index, Slot { done: false, state });
        Ok(Vec::new())
    }

    fn item_done(&mut self, data: &Value) -> Result<Vec<ProviderEvent>, PayloadError> {
        let event = EventType::ItemDone;
        let index = event.index(data, "output_index")?;
        let item = event.object(data, "item")?;
        let kind = event.text(item, "type")?;
        let slot = self
            .items
            .get_mut(&index)
            .ok_or_else(|| event.fail("the item was not added"))?;
        if slot.done {
            return Err(event.fail("the item is done twice"));
        }
        slot.done = true;
        match (&mut slot.state, kind) {
            (ItemState::Message(message), "message") => message::done(
                message,
                index,
                item,
                self.structured,
                &mut self.refusal_seen,
            ),
            (ItemState::Reasoning(reasoning), "reasoning") => {
                reasoning::done(reasoning, index, item)
            }
            (ItemState::Call(call), "function_call") => call::done(call, item),
            _ => Err(event.fail("the done item does not fit the item that was added")),
        }
    }
}

fn open_slot(items: &mut Items, event: EventType, index: u64) -> Result<&mut Slot, PayloadError> {
    let slot = items
        .get_mut(&index)
        .ok_or_else(|| event.fail("the item was not added"))?;
    if slot.done {
        return Err(event.fail("the item is already done"));
    }
    Ok(slot)
}

impl PayloadDecoder for ResponsesDecoder {
    fn payload(&mut self, payload: Payload) -> Result<Vec<ProviderEvent>, PayloadError> {
        if payload.data().trim().is_empty() {
            return Ok(Vec::new());
        }
        let data = serde_json::from_str::<Value>(payload.data())
            .map_err(|_| PayloadError::new("the payload is not valid JSON"))?;
        let name = data
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| PayloadError::new("the payload has no type text"))?;
        match EventType::parse(name) {
            Some(event) => self.handle(event, &data),
            None => Ok(Vec::new()),
        }
    }

    fn end(&mut self) -> Result<End, PayloadError> {
        Ok(End::MarkerMissing)
    }
}
