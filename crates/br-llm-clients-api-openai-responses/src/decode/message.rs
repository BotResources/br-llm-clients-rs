use std::collections::btree_map::Entry;

use br_llm_clients_common_call::{PayloadError, ProviderBlock, ProviderEvent};
use serde_json::Value;

use super::event::EventType;
use super::items::{ItemState, MessageItem, Part, PartKind};
use super::{ResponsesDecoder, open_slot};

pub(super) fn key(output_index: u64, content_index: u64) -> String {
    format!("{output_index}.{content_index}")
}

fn is_text_block(kind: PartKind, structured: bool) -> bool {
    kind == PartKind::Refusal || !structured
}

fn block_of(kind: PartKind, structured: bool) -> ProviderBlock {
    if is_text_block(kind, structured) {
        ProviderBlock::Text
    } else {
        ProviderBlock::Structured
    }
}

fn delta_of(kind: PartKind, structured: bool, key: String, text: &str) -> ProviderEvent {
    if is_text_block(kind, structured) {
        ProviderEvent::TextDelta {
            key,
            text: text.to_owned(),
        }
    } else {
        ProviderEvent::StructuredDelta {
            key,
            json: text.to_owned(),
        }
    }
}

fn start_of(
    output_index: u64,
    content_index: u64,
    kind: PartKind,
    structured: bool,
) -> ProviderEvent {
    ProviderEvent::BlockStart {
        key: key(output_index, content_index),
        block: block_of(kind, structured),
    }
}

impl ResponsesDecoder {
    pub(super) fn part_added(&mut self, data: &Value) -> Result<Vec<ProviderEvent>, PayloadError> {
        let event = EventType::PartAdded;
        let output_index = event.index(data, "output_index")?;
        let content_index = event.index(data, "content_index")?;
        let part = event.object(data, "part")?;
        let part_type = event.text(part, "type")?;
        let structured = self.structured;
        let slot = open_slot(&mut self.items, event, output_index)?;
        match &mut slot.state {
            ItemState::Message(message) => {
                let kind = match part_type {
                    "output_text" => PartKind::Output,
                    "refusal" => PartKind::Refusal,
                    _ => return Err(event.fail("the content part is of a kind that is not known")),
                };
                if message.parts.contains_key(&content_index) {
                    return Err(event.fail("the content part was added twice"));
                }
                message.parts.insert(content_index, Part::new(kind));
                self.refusal_seen = self.refusal_seen || kind == PartKind::Refusal;
                Ok(vec![start_of(
                    output_index,
                    content_index,
                    kind,
                    structured,
                )])
            }
            ItemState::Reasoning(_) if part_type == "reasoning_text" => Ok(Vec::new()),
            ItemState::Reasoning(_) => {
                Err(event
                    .fail("the content part of a reasoning item is of a kind that is not known"))
            }
            ItemState::Call(_) => {
                Err(event.fail("a content part cannot be added to a function call"))
            }
        }
    }

    pub(super) fn text_delta(
        &mut self,
        event: EventType,
        data: &Value,
    ) -> Result<Vec<ProviderEvent>, PayloadError> {
        let kind = if event == EventType::RefusalDelta {
            PartKind::Refusal
        } else {
            PartKind::Output
        };
        let output_index = event.index(data, "output_index")?;
        let content_index = event.index(data, "content_index")?;
        let delta = event.text(data, "delta")?;
        let structured = self.structured;
        let slot = open_slot(&mut self.items, event, output_index)?;
        let ItemState::Message(message) = &mut slot.state else {
            return Err(event.fail("the delta does not fit the item"));
        };
        let mut events = Vec::new();
        let part = match message.parts.entry(content_index) {
            Entry::Occupied(found) => {
                let part = found.into_mut();
                if part.kind != kind {
                    return Err(event.fail("the delta does not fit the content part"));
                }
                part
            }
            Entry::Vacant(vacant) => {
                events.push(start_of(output_index, content_index, kind, structured));
                vacant.insert(Part::new(kind))
            }
        };
        part.streamed.push_str(delta);
        part.delta_given = true;
        self.refusal_seen = self.refusal_seen || kind == PartKind::Refusal;
        events.push(delta_of(
            kind,
            structured,
            key(output_index, content_index),
            delta,
        ));
        Ok(events)
    }
}

pub(super) fn done(
    message: &mut MessageItem,
    output_index: u64,
    item: &Value,
    structured: bool,
    refusal_seen: &mut bool,
) -> Result<Vec<ProviderEvent>, PayloadError> {
    let event = EventType::ItemDone;
    let completed = item.get("status").and_then(Value::as_str) == Some("completed");
    let content = event.optional_array(item, "content")?;
    let mut events = Vec::new();
    for (position, part) in content.iter().enumerate() {
        let content_index = u64::try_from(position).unwrap_or(u64::MAX);
        let (kind, field) = match event.text(part, "type")? {
            "output_text" => (PartKind::Output, "text"),
            "refusal" => (PartKind::Refusal, "refusal"),
            _ => return Err(event.fail("a content part is of a kind that is not known")),
        };
        let text = event.text(part, field)?;
        let known = match message.parts.entry(content_index) {
            Entry::Occupied(found) => {
                let known = found.into_mut();
                if known.kind != kind {
                    return Err(event.fail("a content part does not fit the one that was streamed"));
                }
                known
            }
            Entry::Vacant(vacant) => {
                events.push(start_of(output_index, content_index, kind, structured));
                vacant.insert(Part::new(kind))
            }
        };
        let block_key = key(output_index, content_index);
        if known.delta_given {
            if known.streamed != text {
                return Err(event.fail("a text differs from its done item"));
            }
        } else {
            events.push(delta_of(kind, structured, block_key.clone(), text));
        }
        *refusal_seen = *refusal_seen || kind == PartKind::Refusal;
        if is_text_block(kind, structured) || completed {
            events.push(ProviderEvent::BlockEnd { key: block_key });
        }
    }
    let kept = u64::try_from(content.len()).unwrap_or(u64::MAX);
    if message.parts.keys().any(|index| *index >= kept) {
        return Err(event.fail("a content part is missing from the done item"));
    }
    Ok(events)
}

pub(super) fn finish_undone(
    message: &MessageItem,
    output_index: u64,
    structured: bool,
) -> Vec<ProviderEvent> {
    message
        .parts
        .iter()
        .filter(|(_, part)| is_text_block(part.kind, structured))
        .map(|(content_index, _)| ProviderEvent::BlockEnd {
            key: key(output_index, *content_index),
        })
        .collect()
}
