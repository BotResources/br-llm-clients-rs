use br_llm_clients_core::{ClientError, RenderedHistory, RenderedPart, RenderedRole};
use serde_json::Value;

use crate::input_assistant::assistant_items;
use crate::input_user::user_items;

pub(crate) fn invalid_messages(reason: &str) -> ClientError {
    ClientError::InvalidMessages {
        reason: reason.to_owned(),
    }
}

pub(crate) fn items(history: &RenderedHistory) -> Result<Vec<Value>, ClientError> {
    let mut items = Vec::new();
    let mut pending: Vec<&RenderedPart> = Vec::new();
    for message in history.messages() {
        match message.role() {
            RenderedRole::User => {
                pending.extend(message.parts());
                user_items(&pending)?;
            }
            RenderedRole::Assistant => {
                let found = assistant_items(message.parts())?;
                if !found.is_empty() {
                    items.extend(user_items(&pending)?);
                    pending.clear();
                    items.extend(found);
                }
            }
        }
    }
    if history
        .messages()
        .last()
        .is_some_and(|message| message.role() == RenderedRole::Assistant)
    {
        return Err(invalid_messages(
            "the last message is an assistant message, so the model has nothing to answer",
        ));
    }
    items.extend(user_items(&pending)?);
    Ok(items)
}
