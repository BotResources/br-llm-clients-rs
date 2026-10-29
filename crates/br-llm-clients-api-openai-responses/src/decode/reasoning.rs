use br_llm_clients_common_call::{PayloadError, ProviderBlock, ProviderEvent};
use serde_json::Value;

use super::event::EventType;
use super::items::{ItemState, ReasoningItem};
use super::{ResponsesDecoder, open_slot};
use crate::envelope;

const SUMMARY_SEPARATOR: &str = "\n\n";

fn key(output_index: u64) -> String {
    output_index.to_string()
}

fn open(reasoning: &mut ReasoningItem, output_index: u64, events: &mut Vec<ProviderEvent>) {
    if !reasoning.open {
        reasoning.open = true;
        events.push(ProviderEvent::BlockStart {
            key: key(output_index),
            block: ProviderBlock::Thinking,
        });
    }
}

impl ResponsesDecoder {
    pub(super) fn thinking_delta(
        &mut self,
        event: EventType,
        data: &Value,
    ) -> Result<Vec<ProviderEvent>, PayloadError> {
        let output_index = event.index(data, "output_index")?;
        let summary_index = match event {
            EventType::SummaryDelta => Some(event.index(data, "summary_index")?),
            _ => None,
        };
        let delta = event.text(data, "delta")?;
        let slot = open_slot(&mut self.items, event, output_index)?;
        let ItemState::Reasoning(reasoning) = &mut slot.state else {
            return Err(event.fail("the delta does not fit the item"));
        };
        let mut events = Vec::new();
        open(reasoning, output_index, &mut events);
        if let Some(index) = summary_index {
            let starts_new_part = reasoning
                .highest_summary
                .is_some_and(|highest| index > highest);
            if starts_new_part && reasoning.text_given {
                events.push(ProviderEvent::ThinkingDelta {
                    key: key(output_index),
                    text: SUMMARY_SEPARATOR.to_owned(),
                });
            }
            reasoning.highest_summary = Some(
                reasoning
                    .highest_summary
                    .map_or(index, |highest| highest.max(index)),
            );
        }
        reasoning.text_given = reasoning.text_given || !delta.is_empty();
        events.push(ProviderEvent::ThinkingDelta {
            key: key(output_index),
            text: delta.to_owned(),
        });
        Ok(events)
    }
}

fn summary_text(item: &Value) -> Result<String, PayloadError> {
    let event = EventType::ItemDone;
    let mut texts = Vec::new();
    for part in event.optional_array(item, "summary")? {
        if event.text(part, "type")? != "summary_text" {
            return Err(event.fail("a summary part is of a kind that is not known"));
        }
        texts.push(event.text(part, "text")?);
    }
    Ok(texts.join(SUMMARY_SEPARATOR))
}

pub(super) fn done(
    reasoning: &mut ReasoningItem,
    output_index: u64,
    item: &Value,
) -> Result<Vec<ProviderEvent>, PayloadError> {
    let event = EventType::ItemDone;
    let id = event.optional_text(item, "id")?.filter(|id| !id.is_empty());
    let encrypted = event
        .optional_text(item, "encrypted_content")?
        .filter(|content| !content.is_empty());
    let summary = summary_text(item)?;
    let signature = id
        .zip(encrypted)
        .map(|(id, encrypted)| envelope::make(id, encrypted));
    let format = match signature {
        Some(_) => Some(
            envelope::replay_format()
                .ok_or_else(|| event.fail("the replay format cannot be built"))?,
        ),
        None => None,
    };
    let mut events = Vec::new();
    if !reasoning.open && (signature.is_some() || !summary.is_empty()) {
        open(reasoning, output_index, &mut events);
        if !summary.is_empty() {
            events.push(ProviderEvent::ThinkingDelta {
                key: key(output_index),
                text: summary,
            });
        }
    }
    if reasoning.open {
        if let Some((signature, format)) = signature.zip(format) {
            events.push(ProviderEvent::SignatureDelta {
                key: key(output_index),
                format,
                signature,
            });
        }
        events.push(ProviderEvent::BlockEnd {
            key: key(output_index),
        });
    }
    Ok(events)
}

pub(super) fn finish_undone(reasoning: &ReasoningItem, output_index: u64) -> Vec<ProviderEvent> {
    if reasoning.open {
        vec![ProviderEvent::BlockEnd {
            key: key(output_index),
        }]
    } else {
        Vec::new()
    }
}
