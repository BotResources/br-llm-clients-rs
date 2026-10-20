use br_llm_clients_core::{ClientError, ClientRequest};
use br_llm_messages::{AssistantBlock, WireMessage, WireUserBlock};
use serde_json::{Map, Value};

pub(crate) fn send_back_reasoning(
    messages: &mut [Value],
    request: &ClientRequest,
    thinking_is_on: bool,
) -> Result<(), ClientError> {
    let in_progress_from = turn_in_progress_from(request.messages());
    let sources: Vec<(bool, String)> = request
        .messages()
        .iter()
        .enumerate()
        .filter_map(|(index, message)| match message {
            WireMessage::Assistant { content } if bears_content(content) => Some((
                in_progress_from.is_some_and(|from| index >= from),
                joined_thinking(content),
            )),
            WireMessage::Assistant { .. }
            | WireMessage::User { .. }
            | WireMessage::Relay { .. } => None,
        })
        .collect();
    let targets: Vec<&mut Map<String, Value>> = messages
        .iter_mut()
        .filter_map(Value::as_object_mut)
        .filter(|message| message.get("role").and_then(Value::as_str) == Some("assistant"))
        .collect();
    if sources.len() != targets.len() {
        return Err(ClientError::InvalidMessages {
            reason: "the messages of the request and the messages of the body do not match"
                .to_owned(),
        });
    }
    if !thinking_is_on {
        return Ok(());
    }
    for ((in_progress, thinking), target) in sources.into_iter().zip(targets) {
        if in_progress && !thinking.is_empty() {
            target.insert("reasoning_content".to_owned(), Value::from(thinking));
        }
    }
    Ok(())
}

fn turn_in_progress_from(messages: &[WireMessage]) -> Option<usize> {
    messages
        .iter()
        .rposition(is_plain_user_message)
        .map(|index| index.saturating_add(1))
}

fn is_plain_user_message(message: &WireMessage) -> bool {
    match message {
        WireMessage::User { content } => content.iter().all(|block| match block {
            WireUserBlock::Text { .. } | WireUserBlock::Image(_) => true,
            WireUserBlock::ToolResult(_) => false,
        }),
        WireMessage::Assistant { .. } | WireMessage::Relay { .. } => false,
    }
}

fn bears_content(content: &[AssistantBlock]) -> bool {
    content.iter().any(|block| {
        matches!(
            block,
            AssistantBlock::Text { .. }
                | AssistantBlock::Structured { .. }
                | AssistantBlock::ToolCall(_)
        )
    })
}

fn joined_thinking(content: &[AssistantBlock]) -> String {
    content
        .iter()
        .filter_map(|block| match block {
            AssistantBlock::Thinking(thinking) if thinking.signature.is_none() => {
                Some(thinking.text.as_str())
            }
            _ => None,
        })
        .collect()
}
