use br_llm_clients_core::{ClientError, ClientRequest};
use br_llm_messages::WireMessage;
use serde_json::Value;

use crate::encode_assistant::assistant_message;
use crate::encode_parts::{message, text_part};
use crate::encode_user::user_messages;

pub(crate) fn check_ends_with_user(request: &ClientRequest) -> Result<(), ClientError> {
    match request.messages().last() {
        Some(WireMessage::Assistant { .. }) => Err(ClientError::InvalidMessages {
            reason: "the last message is an assistant message and this dialect sends no prefill"
                .to_owned(),
        }),
        _ => Ok(()),
    }
}

pub(crate) fn messages(request: &ClientRequest) -> Result<Vec<Value>, ClientError> {
    let mut encoded = Vec::new();
    if let Some(system) = request.system() {
        let mut system_message = message("system");
        system_message.insert("content".to_owned(), Value::from(system.as_str()));
        encoded.push(Value::Object(system_message));
    }
    for wire in request.messages() {
        match wire {
            WireMessage::User { content } => encoded.extend(user_messages(content)?),
            WireMessage::Assistant { content } => encoded.extend(assistant_message(content)?),
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
