use br_llm_clients_core::ClientError;
use br_llm_messages::{AssistantBlock, ToolCall};
use serde_json::{Map, Value};

use crate::encode_parts::{message, text_part};

pub(crate) fn assistant_message(content: &[AssistantBlock]) -> Result<Option<Value>, ClientError> {
    let mut texts: Vec<String> = Vec::new();
    let mut calls = Vec::new();
    for block in content {
        match block {
            AssistantBlock::Text { text } => texts.push(text.as_str().to_owned()),
            AssistantBlock::Structured { value } => texts.push(value.to_string()),
            AssistantBlock::ToolCall(call) => calls.push(tool_call(call)),
            AssistantBlock::Thinking(_) | AssistantBlock::RedactedThinking(_) => {}
            _ => {
                return Err(ClientError::InvalidMessages {
                    reason: "an assistant block of a kind that this dialect does not know"
                        .to_owned(),
                });
            }
        }
    }
    if texts.is_empty() && calls.is_empty() {
        return Ok(None);
    }
    let mut assistant = message("assistant");
    match texts.as_slice() {
        [] => {}
        [only] => {
            assistant.insert("content".to_owned(), Value::from(only.as_str()));
        }
        many => {
            let parts = many.iter().map(|text| text_part(text)).collect();
            assistant.insert("content".to_owned(), Value::Array(parts));
        }
    }
    if !calls.is_empty() {
        assistant.insert("tool_calls".to_owned(), Value::Array(calls));
    }
    Ok(Some(Value::Object(assistant)))
}

fn tool_call(call: &ToolCall) -> Value {
    let mut function = Map::new();
    function.insert("name".to_owned(), Value::from(call.name.as_str()));
    function.insert(
        "arguments".to_owned(),
        Value::from(call.arguments.to_string()),
    );
    let mut encoded = Map::new();
    encoded.insert("id".to_owned(), Value::from(call.id.as_str()));
    encoded.insert("type".to_owned(), Value::from("function"));
    encoded.insert("function".to_owned(), Value::Object(function));
    Value::Object(encoded)
}
