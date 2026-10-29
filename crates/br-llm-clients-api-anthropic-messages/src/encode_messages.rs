use br_llm_clients_core::{ClientError, RenderedHistory, RenderedMessage, RenderedRole};
use serde_json::{Value, json};

use crate::encode_blocks::{invalid, part_value};

pub(crate) fn encode_messages(history: &RenderedHistory) -> Result<Vec<Value>, ClientError> {
    if history
        .messages()
        .last()
        .is_some_and(|message| message.role() == RenderedRole::Assistant)
    {
        return Err(invalid("the last message is an assistant message"));
    }
    history.messages().iter().map(message_value).collect()
}

fn message_value(message: &RenderedMessage) -> Result<Value, ClientError> {
    let role = match message.role() {
        RenderedRole::User => "user",
        RenderedRole::Assistant => "assistant",
    };
    let content = message
        .parts()
        .iter()
        .map(part_value)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({"role": role, "content": content}))
}
