use br_llm_clients_core::{ClientError, RenderedHistory, RenderedPart, RenderedRole};
use serde_json::Value;

use crate::input_assistant::assistant_items;
use crate::input_user::user_items;

pub(crate) fn invalid_messages(reason: &str) -> ClientError {
    ClientError::InvalidMessages {
        reason: reason.to_owned(),
    }
}

enum Segment<'a> {
    User(Vec<&'a RenderedPart>),
    Assistant(Vec<Value>),
}

fn segments(history: &RenderedHistory) -> Result<Vec<Segment<'_>>, ClientError> {
    let mut segments: Vec<Segment<'_>> = Vec::new();
    for message in history.messages() {
        match message.role() {
            RenderedRole::User => match segments.last_mut() {
                Some(Segment::User(parts)) => parts.extend(message.parts()),
                _ => segments.push(Segment::User(message.parts().iter().collect())),
            },
            RenderedRole::Assistant => {
                let items = assistant_items(message.parts())?;
                if !items.is_empty() {
                    segments.push(Segment::Assistant(items));
                }
            }
        }
    }
    Ok(segments)
}

pub(crate) fn items(history: &RenderedHistory) -> Result<Vec<Value>, ClientError> {
    if history
        .messages()
        .last()
        .is_some_and(|message| message.role() == RenderedRole::Assistant)
    {
        return Err(invalid_messages(
            "the last message is an assistant message, so the model has nothing to answer",
        ));
    }
    let mut items = Vec::new();
    for segment in segments(history)? {
        match segment {
            Segment::User(parts) => items.extend(user_items(&parts)?),
            Segment::Assistant(found) => items.extend(found),
        }
    }
    Ok(items)
}
