use br_llm_clients_core::{ClientError, RenderedPart, RenderedResultPart};
use br_llm_messages::{ImageBlock, ImageSource};
use serde_json::{Value, json};

use crate::input::invalid_messages;

const ERROR_PREFIX: &str = "Error: ";

fn message_item(parts: Vec<Value>) -> Value {
    json!({ "type": "message", "role": "user", "content": parts })
}

fn text_part(text: &str) -> Value {
    json!({ "type": "input_text", "text": text })
}

pub(crate) fn image_part(image: &ImageBlock) -> Result<Value, ClientError> {
    match image.source() {
        ImageSource::Inline {
            media_type, data, ..
        } => {
            let url = format!("data:{};base64,{}", media_type.as_str(), data.as_str());
            Ok(json!({ "type": "input_image", "image_url": url }))
        }
        _ => Err(invalid_messages(
            "an image has a source that this dialect does not know",
        )),
    }
}

fn holds_image(parts: &[RenderedResultPart]) -> bool {
    parts
        .iter()
        .any(|part| matches!(part, RenderedResultPart::Image(_)))
}

fn unknown_result_part() -> ClientError {
    invalid_messages("a tool result holds a part of a kind that this dialect does not know")
}

fn text_output(parts: &[RenderedResultPart], is_error: bool) -> Result<Value, ClientError> {
    let mut texts = Vec::new();
    for part in parts {
        match part {
            RenderedResultPart::Text(text) => texts.push(text.as_str()),
            _ => return Err(unknown_result_part()),
        }
    }
    let joined = texts.join("\n");
    Ok(Value::String(if is_error {
        format!("{ERROR_PREFIX}{joined}")
    } else {
        joined
    }))
}

fn array_output(parts: &[RenderedResultPart], is_error: bool) -> Result<Value, ClientError> {
    let mut output = Vec::new();
    if is_error {
        output.push(text_part(ERROR_PREFIX));
    }
    for part in parts {
        output.push(match part {
            RenderedResultPart::Text(text) => text_part(text),
            RenderedResultPart::Image(image) => image_part(image)?,
            _ => return Err(unknown_result_part()),
        });
    }
    Ok(Value::Array(output))
}

fn tool_result_item(
    wire_id: &str,
    parts: &[RenderedResultPart],
    is_error: bool,
) -> Result<Value, ClientError> {
    let output = if holds_image(parts) {
        array_output(parts, is_error)?
    } else {
        text_output(parts, is_error)?
    };
    Ok(json!({
        "type": "function_call_output",
        "call_id": wire_id,
        "output": output,
    }))
}

pub(crate) fn user_items(parts: &[&RenderedPart]) -> Result<Vec<Value>, ClientError> {
    let mut items = Vec::new();
    let mut run: Vec<Value> = Vec::new();
    for part in parts {
        match part {
            RenderedPart::Text(text) => run.push(text_part(text)),
            RenderedPart::Image(image) => run.push(image_part(image)?),
            RenderedPart::ToolResult {
                wire_id,
                is_error,
                parts,
                ..
            } => {
                if !run.is_empty() {
                    items.push(message_item(std::mem::take(&mut run)));
                }
                items.push(tool_result_item(wire_id, parts, *is_error)?);
            }
            RenderedPart::ToolCall { .. } | RenderedPart::Block(_) => {
                return Err(invalid_messages(
                    "a user message holds a tool call or a block of a model response",
                ));
            }
            _ => {
                return Err(invalid_messages(
                    "a user message holds a part of a kind that this dialect does not know",
                ));
            }
        }
    }
    if !run.is_empty() {
        items.push(message_item(run));
    }
    Ok(items)
}
