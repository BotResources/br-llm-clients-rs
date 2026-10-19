use br_llm_clients_core::ClientError;
use br_llm_messages::{ToolResult, ToolResultBlock, WireUserBlock};
use serde_json::Value;

use crate::encode_parts::{message, text_part};

pub(crate) fn user_messages(content: &[WireUserBlock]) -> Result<Vec<Value>, ClientError> {
    let mut messages = Vec::new();
    for block in content {
        if let WireUserBlock::ToolResult(result) = block {
            messages.push(tool_message(result)?);
        }
    }
    let parts: Vec<&WireUserBlock> = content
        .iter()
        .filter(|block| !matches!(block, WireUserBlock::ToolResult(_)))
        .collect();
    if !parts.is_empty() {
        messages.push(user_message(&parts));
    }
    Ok(messages)
}

fn tool_message(result: &ToolResult) -> Result<Value, ClientError> {
    let mut texts = Vec::new();
    for block in &result.content {
        match block {
            ToolResultBlock::Text { text } => texts.push(text.as_str()),
            _ => {
                return Err(ClientError::InvalidMessages {
                    reason: "a tool result block of a kind that this dialect does not know"
                        .to_owned(),
                });
            }
        }
    }
    let content = match texts.as_slice() {
        [] => Value::from(""),
        [only] => Value::from(*only),
        many => Value::Array(many.iter().map(|text| text_part(text)).collect()),
    };
    let mut tool = message("tool");
    tool.insert(
        "tool_call_id".to_owned(),
        Value::from(result.tool_call_id.as_str()),
    );
    tool.insert("content".to_owned(), content);
    Ok(Value::Object(tool))
}

fn user_message(blocks: &[&WireUserBlock]) -> Value {
    let content = match blocks {
        [WireUserBlock::Text { text }] => Value::from(text.as_str()),
        _ => Value::Array(blocks.iter().filter_map(|block| part(block)).collect()),
    };
    let mut user = message("user");
    user.insert("content".to_owned(), content);
    Value::Object(user)
}

fn part(block: &WireUserBlock) -> Option<Value> {
    match block {
        WireUserBlock::Text { text } => Some(text_part(text.as_str())),
        WireUserBlock::Image(image) => Some(image_part(image.mime.as_str(), image.data.as_str())),
        WireUserBlock::ToolResult(_) => None,
    }
}

fn image_part(mime: &str, data: &str) -> Value {
    let mut url = serde_json::Map::new();
    url.insert(
        "url".to_owned(),
        Value::from(format!("data:{mime};base64,{data}")),
    );
    let mut part = serde_json::Map::new();
    part.insert("type".to_owned(), Value::from("image_url"));
    part.insert("image_url".to_owned(), Value::Object(url));
    Value::Object(part)
}
