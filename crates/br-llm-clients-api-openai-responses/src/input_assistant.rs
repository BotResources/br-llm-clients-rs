use br_llm_clients_core::ClientError;
use br_llm_messages::{AssistantBlock, Thinking, ToolCall};
use serde_json::{Value, json};

use crate::envelope;
use crate::input::invalid_messages;

fn message_item(content: &str) -> Value {
    json!({ "type": "message", "role": "assistant", "content": content })
}

fn call_item(call: &ToolCall) -> Value {
    json!({
        "type": "function_call",
        "call_id": call.id.as_str(),
        "name": call.name.as_str(),
        "arguments": call.arguments.to_string(),
    })
}

fn reasoning_item(thinking: &Thinking) -> Option<Value> {
    let signature = thinking.signature.as_ref()?;
    let envelope = envelope::parse(signature.as_str())?;
    Some(json!({
        "type": "reasoning",
        "id": envelope.id,
        "summary": [],
        "encrypted_content": envelope.encrypted_content,
    }))
}

pub(crate) fn assistant_items(
    content: &[AssistantBlock],
    in_progress: bool,
) -> Result<Vec<Value>, ClientError> {
    let mut items = Vec::new();
    for block in content {
        match block {
            AssistantBlock::Text { text } => items.push(message_item(text.as_str())),
            AssistantBlock::Structured { value } => items.push(message_item(&value.to_string())),
            AssistantBlock::ToolCall(call) => items.push(call_item(call)),
            AssistantBlock::Thinking(thinking) => {
                if in_progress {
                    items.extend(reasoning_item(thinking));
                }
            }
            AssistantBlock::RedactedThinking(_) => {}
            _ => {
                return Err(invalid_messages(
                    "an assistant message holds a block of a kind that this dialect does not know",
                ));
            }
        }
    }
    Ok(items)
}
