use br_llm_clients_core::ClientError;
use br_llm_messages::{Image, Text, ToolResult, ToolResultBlock, WireUserBlock};
use serde_json::{Value, json};

use crate::input::invalid_messages;

fn message_item(parts: Vec<Value>) -> Value {
    json!({ "type": "message", "role": "user", "content": parts })
}

fn text_part(text: &str) -> Value {
    json!({ "type": "input_text", "text": text })
}

fn image_part(image: &Image) -> Value {
    let url = format!(
        "data:{};base64,{}",
        image.mime.as_str(),
        image.data.as_str()
    );
    json!({ "type": "input_image", "image_url": url })
}

pub(crate) fn relay_item(texts: &[Text]) -> Value {
    message_item(texts.iter().map(|text| text_part(text.as_str())).collect())
}

fn tool_output(result: &ToolResult) -> Result<String, ClientError> {
    let mut texts = Vec::new();
    for block in &result.content {
        match block {
            ToolResultBlock::Text { text } => texts.push(text.as_str()),
            _ => {
                return Err(invalid_messages(
                    "a tool result holds a block of a kind that this dialect does not know",
                ));
            }
        }
    }
    let joined = texts.join("\n");
    Ok(if result.is_error {
        format!("Error: {joined}")
    } else {
        joined
    })
}

fn tool_result_item(result: &ToolResult) -> Result<Value, ClientError> {
    Ok(json!({
        "type": "function_call_output",
        "call_id": result.tool_call_id.as_str(),
        "output": tool_output(result)?,
    }))
}

pub(crate) fn user_items(content: &[WireUserBlock]) -> Result<Vec<Value>, ClientError> {
    let mut items = Vec::new();
    let mut run: Vec<Value> = Vec::new();
    for block in content {
        match block {
            WireUserBlock::Text { text } => run.push(text_part(text.as_str())),
            WireUserBlock::Image(image) => run.push(image_part(image)),
            WireUserBlock::ToolResult(result) => {
                if !run.is_empty() {
                    items.push(message_item(std::mem::take(&mut run)));
                }
                items.push(tool_result_item(result)?);
            }
        }
    }
    if !run.is_empty() {
        items.push(message_item(run));
    }
    Ok(items)
}
