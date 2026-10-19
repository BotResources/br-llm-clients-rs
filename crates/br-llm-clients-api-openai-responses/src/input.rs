use br_llm_clients_core::ClientError;
use br_llm_messages::WireMessage;
use serde_json::Value;

use crate::input_assistant::assistant_items;
use crate::input_user::{relay_item, user_items};
use crate::turn::{is_in_progress, turn_start};

pub(crate) fn invalid_messages(reason: &str) -> ClientError {
    ClientError::InvalidMessages {
        reason: reason.to_owned(),
    }
}

pub(crate) fn items(messages: &[WireMessage]) -> Result<Vec<Value>, ClientError> {
    if matches!(messages.last(), Some(WireMessage::Assistant { .. })) {
        return Err(invalid_messages(
            "the last message is an assistant message, so the model has nothing to answer",
        ));
    }
    let start = turn_start(messages);
    let mut items = Vec::new();
    for (index, message) in messages.iter().enumerate() {
        match message {
            WireMessage::User { content } => items.extend(user_items(content)?),
            WireMessage::Assistant { content } => {
                items.extend(assistant_items(content, is_in_progress(start, index))?);
            }
            WireMessage::Relay { content } => items.push(relay_item(content)),
        }
    }
    Ok(items)
}
