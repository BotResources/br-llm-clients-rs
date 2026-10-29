use std::collections::HashMap;

use br_llm_clients_common_call::{PayloadError, ProviderBlock, ProviderEvent};
use serde_json::Value;

use crate::decode_fields::{
    Fields, index_of, malformed, non_empty_text, object_field, optional_text, required_text,
    type_of,
};
use crate::replay_formats::{REDACTED_DATA_FORMAT, SIGNATURE_FORMAT, replay_format};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Open {
    Text,
    Structured,
    Thinking,
    RedactedThinking,
    ToolCall { received_arguments: bool },
}

#[derive(Debug, Default)]
pub(crate) struct Blocks {
    structured: bool,
    open: HashMap<u64, Open>,
    held: Vec<u64>,
}

impl Blocks {
    pub(crate) fn new(structured: bool) -> Self {
        Self {
            structured,
            ..Self::default()
        }
    }

    pub(crate) fn start(&mut self, fields: &Fields) -> Result<Vec<ProviderEvent>, PayloadError> {
        let index = index_of(fields)?;
        let block = object_field(fields, "content_block")?;
        let kind = type_of(block, "content block")?;
        if self.open.contains_key(&index) || self.held.contains(&index) {
            return Err(malformed("a block starts at an index that is still in use"));
        }
        let key = index.to_string();
        let (open, events) = match kind {
            "text" => self.start_text(&key, block)?,
            "thinking" => start_thinking(&key, block)?,
            "redacted_thinking" => start_redacted(&key, block)?,
            "tool_use" => start_tool_call(&key, block)?,
            _ => return Err(malformed("a content block of a type that is not accepted")),
        };
        self.open.insert(index, open);
        Ok(events)
    }

    fn start_text(
        &self,
        key: &str,
        block: &Fields,
    ) -> Result<(Open, Vec<ProviderEvent>), PayloadError> {
        let text = optional_text(block, "text")?.filter(|text| !text.is_empty());
        let key = key.to_owned();
        if self.structured {
            let mut events = vec![ProviderEvent::BlockStart {
                key: key.clone(),
                block: ProviderBlock::Structured,
            }];
            events.extend(text.map(|json| ProviderEvent::StructuredDelta {
                key,
                json: json.to_owned(),
            }));
            return Ok((Open::Structured, events));
        }
        let mut events = vec![ProviderEvent::BlockStart {
            key: key.clone(),
            block: ProviderBlock::Text,
        }];
        events.extend(text.map(|text| ProviderEvent::TextDelta {
            key,
            text: text.to_owned(),
        }));
        Ok((Open::Text, events))
    }

    pub(crate) fn delta(&mut self, fields: &Fields) -> Result<Vec<ProviderEvent>, PayloadError> {
        let index = index_of(fields)?;
        let delta = object_field(fields, "delta")?;
        let kind = type_of(delta, "delta")?;
        let open = self
            .open
            .get_mut(&index)
            .ok_or_else(|| malformed("a delta for a block that is not open"))?;
        let key = index.to_string();
        let event = match (kind, &mut *open) {
            ("text_delta", Open::Text) => {
                let text = required_text(delta, "text")?.to_owned();
                (!text.is_empty()).then_some(ProviderEvent::TextDelta { key, text })
            }
            ("text_delta", Open::Structured) => {
                let json = required_text(delta, "text")?.to_owned();
                (!json.is_empty()).then_some(ProviderEvent::StructuredDelta { key, json })
            }
            ("thinking_delta", Open::Thinking) => {
                let text = required_text(delta, "thinking")?.to_owned();
                (!text.is_empty()).then_some(ProviderEvent::ThinkingDelta { key, text })
            }
            ("signature_delta", Open::Thinking) => {
                let signature = required_text(delta, "signature")?.to_owned();
                if signature.is_empty() {
                    None
                } else {
                    Some(ProviderEvent::SignatureDelta {
                        key,
                        format: replay_format(SIGNATURE_FORMAT)?,
                        signature,
                    })
                }
            }
            ("input_json_delta", Open::ToolCall { received_arguments }) => {
                let json = required_text(delta, "partial_json")?.to_owned();
                if json.is_empty() {
                    None
                } else {
                    *received_arguments = true;
                    Some(ProviderEvent::ToolArgumentsDelta { key, json })
                }
            }
            _ => {
                return Err(malformed(
                    "a delta of a type that is unknown or does not fit its block",
                ));
            }
        };
        Ok(event.into_iter().collect())
    }

    pub(crate) fn stop(&mut self, fields: &Fields) -> Result<Vec<ProviderEvent>, PayloadError> {
        let index = index_of(fields)?;
        let open = self
            .open
            .remove(&index)
            .ok_or_else(|| malformed("a stop for a block that is not open"))?;
        if open
            == (Open::ToolCall {
                received_arguments: false,
            })
        {
            self.held.push(index);
            return Ok(Vec::new());
        }
        Ok(vec![ProviderEvent::BlockEnd {
            key: index.to_string(),
        }])
    }

    pub(crate) fn release_held(&mut self) -> Vec<ProviderEvent> {
        std::mem::take(&mut self.held)
            .into_iter()
            .flat_map(|index| {
                let key = index.to_string();
                [
                    ProviderEvent::ToolArgumentsDelta {
                        key: key.clone(),
                        json: "{}".to_owned(),
                    },
                    ProviderEvent::BlockEnd { key },
                ]
            })
            .collect()
    }

    pub(crate) fn drop_held(&mut self) {
        self.held.clear();
    }
}

fn start_thinking(key: &str, block: &Fields) -> Result<(Open, Vec<ProviderEvent>), PayloadError> {
    let thinking = optional_text(block, "thinking")?.filter(|text| !text.is_empty());
    let signature = optional_text(block, "signature")?.filter(|text| !text.is_empty());
    let mut events = vec![ProviderEvent::BlockStart {
        key: key.to_owned(),
        block: ProviderBlock::Thinking,
    }];
    events.extend(thinking.map(|text| ProviderEvent::ThinkingDelta {
        key: key.to_owned(),
        text: text.to_owned(),
    }));
    if let Some(signature) = signature {
        events.push(ProviderEvent::SignatureDelta {
            key: key.to_owned(),
            format: replay_format(SIGNATURE_FORMAT)?,
            signature: signature.to_owned(),
        });
    }
    Ok((Open::Thinking, events))
}

fn start_redacted(key: &str, block: &Fields) -> Result<(Open, Vec<ProviderEvent>), PayloadError> {
    let data = non_empty_text(block, "data")?;
    Ok((
        Open::RedactedThinking,
        vec![
            ProviderEvent::BlockStart {
                key: key.to_owned(),
                block: ProviderBlock::RedactedThinking,
            },
            ProviderEvent::RedactedThinkingData {
                key: key.to_owned(),
                format: replay_format(REDACTED_DATA_FORMAT)?,
                data: data.to_owned(),
            },
        ],
    ))
}

fn start_tool_call(key: &str, block: &Fields) -> Result<(Open, Vec<ProviderEvent>), PayloadError> {
    let name = non_empty_text(block, "name")?;
    let id = non_empty_text(block, "id")?;
    match block.get("input") {
        None => {}
        Some(Value::Object(input)) if input.is_empty() => {}
        Some(_) => {
            return Err(malformed(
                "the input of a tool call start is neither absent nor an empty object",
            ));
        }
    }
    Ok((
        Open::ToolCall {
            received_arguments: false,
        },
        vec![ProviderEvent::BlockStart {
            key: key.to_owned(),
            block: ProviderBlock::ToolCall {
                name: name.to_owned(),
                id: Some(id.to_owned()),
            },
        }],
    ))
}
