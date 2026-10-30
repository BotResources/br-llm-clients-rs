use br_llm_clients_core::{ClientError, RenderedPart, RenderedResultPart};
use br_llm_messages::{
    ContentBlock, ImageBlock, ImageSource, MediaType, RedactedThinkingBlock, RuntimeToolCall,
    ThinkingBlock,
};
use serde_json::{Map, Value, json};

use crate::replay_formats::{REDACTED_DATA_FORMAT, SIGNATURE_FORMAT, text_in_format};

pub(crate) fn invalid(reason: impl Into<String>) -> ClientError {
    ClientError::InvalidMessages {
        reason: reason.into(),
    }
}

pub(crate) fn text_block(text: &str) -> Value {
    json!({"type": "text", "text": text})
}

pub(crate) fn part_value(part: &RenderedPart) -> Result<Value, ClientError> {
    match part {
        RenderedPart::Text(text) => Ok(text_block(text)),
        RenderedPart::Image(image) => image_block(image),
        RenderedPart::ToolCall { call, wire_id } => tool_use(call, wire_id),
        RenderedPart::ToolResult {
            wire_id,
            is_error,
            parts,
            ..
        } => tool_result(wire_id, *is_error, parts),
        RenderedPart::Block(block) => content_block(block),
        _ => Err(invalid(
            "a part of a kind that this version does not encode",
        )),
    }
}

fn image_block(image: &ImageBlock) -> Result<Value, ClientError> {
    let (media_type, data) = match image.source() {
        ImageSource::Inline {
            media_type, data, ..
        } => (media_type, data),
        _ => {
            return Err(invalid(
                "an image source of a kind that this version does not encode",
            ));
        }
    };
    let media_type = match media_type {
        MediaType::Jpeg | MediaType::Png | MediaType::Gif | MediaType::Webp => media_type.as_str(),
        _ => {
            return Err(invalid(format!(
                "an image of media type {} that this version does not know",
                media_type.as_str()
            )));
        }
    };
    Ok(json!({
        "type": "image",
        "source": {"type": "base64", "media_type": media_type, "data": data.as_str()},
    }))
}

fn tool_use(call: &RuntimeToolCall, wire_id: &str) -> Result<Value, ClientError> {
    let input = call.arguments().deserialize_into::<Value>().map_err(|_| {
        invalid(format!(
            "the arguments of tool call {wire_id} cannot be read"
        ))
    })?;
    Ok(json!({
        "type": "tool_use",
        "id": wire_id,
        "name": call.name().as_str(),
        "input": input,
    }))
}

fn tool_result(
    wire_id: &str,
    is_error: bool,
    parts: &[RenderedResultPart],
) -> Result<Value, ClientError> {
    let mut block = Map::new();
    block.insert("type".to_owned(), json!("tool_result"));
    block.insert("tool_use_id".to_owned(), json!(wire_id));
    if !parts.is_empty() {
        let content = parts
            .iter()
            .map(|part| match part {
                RenderedResultPart::Text(text) => Ok(text_block(text)),
                RenderedResultPart::Image(image) => image_block(image),
                _ => Err(invalid(
                    "a tool result part of a kind that this version does not encode",
                )),
            })
            .collect::<Result<Vec<_>, _>>()?;
        block.insert("content".to_owned(), Value::Array(content));
    }
    if is_error {
        block.insert("is_error".to_owned(), Value::Bool(true));
    }
    Ok(Value::Object(block))
}

fn content_block(block: &ContentBlock) -> Result<Value, ClientError> {
    match block {
        ContentBlock::Text(text) => Ok(text_block(text.text().as_str())),
        ContentBlock::Thinking(thinking) => thinking_block(thinking),
        ContentBlock::RedactedThinking(redacted) => redacted_block(redacted),
        other => Err(invalid(format!(
            "a content block of kind {} that this version does not encode",
            other.kind().as_str()
        ))),
    }
}

fn thinking_block(thinking: &ThinkingBlock) -> Result<Value, ClientError> {
    let signature = thinking
        .replay()
        .and_then(|material| text_in_format(material, SIGNATURE_FORMAT))
        .ok_or_else(|| {
            invalid("a thinking block of the turn in progress has no signature in the format of this library")
        })?;
    Ok(json!({
        "type": "thinking",
        "thinking": thinking.text(),
        "signature": signature,
    }))
}

fn redacted_block(redacted: &RedactedThinkingBlock) -> Result<Value, ClientError> {
    let data = text_in_format(redacted.material(), REDACTED_DATA_FORMAT).ok_or_else(|| {
        invalid("a redacted thinking block of the turn in progress has no data in the format of this library")
    })?;
    Ok(json!({"type": "redacted_thinking", "data": data}))
}
