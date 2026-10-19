use br_llm_clients_core::ClientError;
use br_llm_messages::{AssistantBlock, ToolResult, ToolResultBlock, WireUserBlock};
use serde_json::{Map, Value, json};

pub(crate) fn invalid(reason: impl Into<String>) -> ClientError {
    ClientError::InvalidMessages {
        reason: reason.into(),
    }
}

pub(crate) fn text_block(text: &str) -> Value {
    json!({"type": "text", "text": text})
}

pub(crate) fn user_block(block: &WireUserBlock) -> Result<Value, ClientError> {
    match block {
        WireUserBlock::Text { text } => Ok(text_block(text.as_str())),
        WireUserBlock::Image(image) => Ok(json!({
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": image.mime.as_str(),
                "data": image.data.as_str(),
            },
        })),
        WireUserBlock::ToolResult(result) => tool_result(result),
    }
}

fn tool_result(result: &ToolResult) -> Result<Value, ClientError> {
    let mut block = Map::new();
    block.insert("type".to_owned(), json!("tool_result"));
    block.insert(
        "tool_use_id".to_owned(),
        json!(result.tool_call_id.as_str()),
    );
    if !result.content.is_empty() {
        let content = result
            .content
            .iter()
            .map(|part| match part {
                ToolResultBlock::Text { text } => Ok(text_block(text.as_str())),
                _ => Err(invalid(
                    "a tool result block of a kind that this version does not know",
                )),
            })
            .collect::<Result<Vec<_>, _>>()?;
        block.insert("content".to_owned(), Value::Array(content));
    }
    if result.is_error {
        block.insert("is_error".to_owned(), Value::Bool(true));
    }
    Ok(Value::Object(block))
}

pub(crate) fn assistant_block(
    block: &AssistantBlock,
    in_turn_in_progress: bool,
) -> Result<Option<Value>, ClientError> {
    match block {
        AssistantBlock::Text { text } => Ok(Some(text_block(text.as_str()))),
        AssistantBlock::Structured { value } => Ok(Some(text_block(&value.to_string()))),
        AssistantBlock::ToolCall(call) => {
            if !call.arguments.is_object() {
                return Err(invalid(format!(
                    "the arguments of tool call {} are not a JSON object",
                    call.id
                )));
            }
            Ok(Some(json!({
                "type": "tool_use",
                "id": call.id.as_str(),
                "name": call.name.as_str(),
                "input": call.arguments,
            })))
        }
        AssistantBlock::Thinking(thinking) => {
            if !in_turn_in_progress {
                return Ok(None);
            }
            let signature = thinking.signature.as_ref().ok_or_else(|| {
                invalid("a thinking block of the turn in progress has no signature")
            })?;
            Ok(Some(json!({
                "type": "thinking",
                "thinking": thinking.text,
                "signature": signature.as_str(),
            })))
        }
        AssistantBlock::RedactedThinking(redacted) => {
            if !in_turn_in_progress {
                return Ok(None);
            }
            Ok(Some(
                json!({"type": "redacted_thinking", "data": redacted.data()}),
            ))
        }
        _ => Err(invalid(
            "an assistant block of a kind that this version does not know",
        )),
    }
}
