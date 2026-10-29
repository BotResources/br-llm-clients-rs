use br_llm_clients_core::{ClientError, ClientRequest};
use br_llm_messages::{WireMessage, WireUserBlock};
use serde_json::Value;

use crate::encode_assistant::assistant_message;
use crate::encode_parts::{message, text_part};
use crate::encode_user::user_messages;
use crate::options::ChatCompletionsOptions;

pub(crate) fn check_ends_with_user(request: &ClientRequest) -> Result<(), ClientError> {
    match request.messages().last() {
        Some(WireMessage::Assistant { .. }) => Err(ClientError::InvalidMessages {
            reason: "the last message is an assistant message and this dialect sends no prefill"
                .to_owned(),
        }),
        _ => Ok(()),
    }
}

fn is_turn_boundary(message: &WireMessage) -> bool {
    match message {
        WireMessage::User { content } => !content
            .iter()
            .any(|block| matches!(block, WireUserBlock::ToolResult(_))),
        _ => false,
    }
}

pub(crate) fn messages(
    request: &ClientRequest,
    options: &ChatCompletionsOptions,
) -> Result<Vec<Value>, ClientError> {
    let boundary = request.messages().iter().rposition(is_turn_boundary);
    let mut encoded = Vec::new();
    if let Some(system) = request.system() {
        let mut system_message = message("system");
        system_message.insert("content".to_owned(), Value::from(system.as_str()));
        encoded.push(Value::Object(system_message));
    }
    for (index, wire) in request.messages().iter().enumerate() {
        match wire {
            WireMessage::User { content } => encoded.extend(user_messages(content, options)?),
            WireMessage::Assistant { content } => {
                let in_turn = boundary.is_none_or(|last| index > last);
                encoded.extend(assistant_message(content, options, in_turn)?)
            }
            WireMessage::Relay { content } => encoded.push(relay_message(content)),
        }
    }
    Ok(encoded)
}

fn relay_message(texts: &[br_llm_messages::Text]) -> Value {
    let parts = texts.iter().map(|text| text_part(text.as_str())).collect();
    let mut relay = message("user");
    relay.insert("content".to_owned(), Value::Array(parts));
    Value::Object(relay)
}
