use br_llm_clients_common_call::{PayloadError, ProviderBlock, ProviderEvent};
use serde_json::{Map, Value};

use super::ChatCompletionsDecoder;
use crate::json_fields::{array_field, present, text_field, wrong};
use crate::tool_fragments::hold;

const REASONING_KEY: &str = "reasoning";
const TEXT_KEY: &str = "text";
const STRUCTURED_KEY: &str = "structured";
const REFUSAL_KEY: &str = "refusal";

struct DeltaText<'a> {
    reasoning: &'a str,
    content: &'a str,
    refusal: &'a str,
    fragments: &'a [Value],
}

impl ChatCompletionsDecoder {
    pub(super) fn read_delta(
        &mut self,
        delta: &Map<String, Value>,
        events: &mut Vec<ProviderEvent>,
    ) -> Result<(), PayloadError> {
        reject_unrepresentable(delta)?;
        let reasoning = self.reasoning_text(delta)?;
        let content = text_field(delta, "content", "choices[].delta.content")?.unwrap_or_default();
        let refusal = text_field(delta, "refusal", "choices[].delta.refusal")?.unwrap_or_default();
        let fragments =
            array_field(delta, "tool_calls", "choices[].delta.tool_calls")?.unwrap_or_default();
        let text = DeltaText {
            reasoning,
            content,
            refusal,
            fragments,
        };
        if self.finished {
            reject_after_finish(&text)?;
        }
        self.emit(&text, events)
    }

    fn reasoning_text<'a>(&self, delta: &'a Map<String, Value>) -> Result<&'a str, PayloadError> {
        let Some(field) = self.reasoning_field.as_deref() else {
            return Ok("");
        };
        match present(delta, field) {
            None => Ok(""),
            Some(Value::String(text)) => Ok(text.as_str()),
            Some(_) => Err(wrong(&format!("choices[].delta.{field}"), "a string")),
        }
    }

    fn emit(
        &mut self,
        text: &DeltaText<'_>,
        events: &mut Vec<ProviderEvent>,
    ) -> Result<(), PayloadError> {
        if !text.reasoning.is_empty() {
            self.open_block(REASONING_KEY, ProviderBlock::Thinking, events);
            events.push(ProviderEvent::ThinkingDelta {
                key: REASONING_KEY.to_owned(),
                text: text.reasoning.to_owned(),
            });
        }
        if !text.content.is_empty() {
            self.emit_content(text.content, events);
        }
        if !text.refusal.is_empty() {
            self.refusal_seen = true;
            self.open_block(REFUSAL_KEY, ProviderBlock::Text, events);
            events.push(ProviderEvent::TextDelta {
                key: REFUSAL_KEY.to_owned(),
                text: text.refusal.to_owned(),
            });
        }
        hold(&mut self.held, text.fragments)
    }

    fn emit_content(&mut self, content: &str, events: &mut Vec<ProviderEvent>) {
        if self.structured {
            self.open_block(STRUCTURED_KEY, ProviderBlock::Structured, events);
            events.push(ProviderEvent::StructuredDelta {
                key: STRUCTURED_KEY.to_owned(),
                json: content.to_owned(),
            });
        } else {
            self.open_block(TEXT_KEY, ProviderBlock::Text, events);
            events.push(ProviderEvent::TextDelta {
                key: TEXT_KEY.to_owned(),
                text: content.to_owned(),
            });
        }
    }

    pub(super) fn close_open_block(&mut self, events: &mut Vec<ProviderEvent>) {
        if let Some(key) = self.open.take() {
            events.push(ProviderEvent::BlockEnd {
                key: key.to_owned(),
            });
        }
    }

    fn open_block(
        &mut self,
        key: &'static str,
        block: ProviderBlock,
        events: &mut Vec<ProviderEvent>,
    ) {
        if self.open == Some(key) {
            return;
        }
        self.close_open_block(events);
        events.push(ProviderEvent::BlockStart {
            key: key.to_owned(),
            block,
        });
        self.open = Some(key);
    }
}

fn reject_unrepresentable(delta: &Map<String, Value>) -> Result<(), PayloadError> {
    for field in ["audio", "function_call"] {
        if present(delta, field).is_some() {
            return Err(PayloadError::new(format!(
                "the delta holds the field `{field}`, which carries output that this dialect cannot represent"
            )));
        }
    }
    Ok(())
}

fn reject_after_finish(text: &DeltaText<'_>) -> Result<(), PayloadError> {
    let output_after_finish = !text.reasoning.is_empty()
        || !text.content.is_empty()
        || !text.refusal.is_empty()
        || !text.fragments.is_empty();
    if output_after_finish {
        return Err(PayloadError::new(
            "the delta holds output after the finish reason",
        ));
    }
    Ok(())
}
