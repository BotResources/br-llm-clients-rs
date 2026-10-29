use br_llm_clients_core::{ClientError, RenderedPart};
use br_llm_messages::{ContentBlock, RuntimeToolCall, ThinkingBlock};
use serde_json::{Value, json};

use crate::envelope;
use crate::input::invalid_messages;

fn message_item(content: &str) -> Value {
    json!({ "type": "message", "role": "assistant", "content": content })
}

fn call_item(call: &RuntimeToolCall, wire_id: &str) -> Result<Value, ClientError> {
    let arguments = call
        .arguments()
        .to_json_string()
        .map_err(|_| invalid_messages("the arguments of a tool call cannot be written as JSON"))?;
    Ok(json!({
        "type": "function_call",
        "call_id": wire_id,
        "name": call.name().as_str(),
        "arguments": arguments,
    }))
}

fn reasoning_item(thinking: &ThinkingBlock) -> Option<Value> {
    let envelope = envelope::of_material(thinking.replay()?)?;
    Some(json!({
        "type": "reasoning",
        "id": envelope.id,
        "summary": [],
        "encrypted_content": envelope.encrypted_content,
    }))
}

fn block_item(block: &ContentBlock) -> Result<Option<Value>, ClientError> {
    match block {
        ContentBlock::Text(text) => Ok(Some(message_item(text.text().as_str()))),
        ContentBlock::Thinking(thinking) => Ok(reasoning_item(thinking)),
        ContentBlock::RedactedThinking(_) => Ok(None),
        other => Err(invalid_messages(&format!(
            "an assistant message holds a block of kind {} that this dialect cannot send",
            other.kind().as_str()
        ))),
    }
}

pub(crate) fn assistant_items(parts: &[RenderedPart]) -> Result<Vec<Value>, ClientError> {
    let mut items = Vec::new();
    for part in parts {
        match part {
            RenderedPart::Text(text) => items.push(message_item(text)),
            RenderedPart::ToolCall { call, wire_id } => items.push(call_item(call, wire_id)?),
            RenderedPart::Block(block) => items.extend(block_item(block)?),
            RenderedPart::Image(_) => {
                return Err(invalid_messages(
                    "an assistant message holds an image, which this dialect cannot send",
                ));
            }
            RenderedPart::ToolResult { .. } => {
                return Err(invalid_messages("an assistant message holds a tool result"));
            }
            _ => {
                return Err(invalid_messages(
                    "an assistant message holds a part of a kind that this dialect does not know",
                ));
            }
        }
    }
    Ok(items)
}
