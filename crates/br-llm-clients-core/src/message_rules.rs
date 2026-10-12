use std::collections::HashSet;

use br_llm_messages::{ToolCallId, WireMessage, WireUserBlock};

use crate::client_error::ClientError;

fn invalid(reason: String) -> ClientError {
    ClientError::InvalidMessages { reason }
}

pub(crate) fn check(messages: &[WireMessage]) -> Result<(), ClientError> {
    if !matches!(messages.first(), Some(WireMessage::User { .. })) {
        return Err(invalid(
            "the list does not start with a user message".to_owned(),
        ));
    }
    let mut seen_calls: HashSet<&ToolCallId> = HashSet::new();
    let mut answered: HashSet<&ToolCallId> = HashSet::new();
    let mut pending: Vec<&ToolCallId> = Vec::new();
    for message in messages {
        match message {
            WireMessage::Assistant { content } => {
                for call in content.iter().filter_map(|block| block.as_tool_call()) {
                    if !seen_calls.insert(&call.id) {
                        return Err(invalid(format!(
                            "tool call id {} appears more than once",
                            call.id
                        )));
                    }
                    pending.push(&call.id);
                }
            }
            WireMessage::User { content } => {
                let results: Vec<&ToolCallId> = content
                    .iter()
                    .filter_map(|block| match block {
                        WireUserBlock::ToolResult(result) => Some(&result.tool_call_id),
                        WireUserBlock::Text { .. } | WireUserBlock::Image(_) => None,
                    })
                    .collect();
                for id in &results {
                    if !seen_calls.contains(id) {
                        return Err(invalid(format!(
                            "tool result {id} matches no earlier tool call"
                        )));
                    }
                    if !answered.insert(id) {
                        return Err(invalid(format!(
                            "tool call {id} is answered more than once"
                        )));
                    }
                }
                for id in pending.drain(..) {
                    if !results.contains(&id) {
                        return Err(invalid(format!(
                            "tool call {id} is not answered in the next user message"
                        )));
                    }
                }
            }
            WireMessage::Relay { .. } => {}
        }
    }
    if let Some(id) = pending.first() {
        return Err(invalid(format!(
            "tool call {id} is not answered: no user message follows"
        )));
    }
    Ok(())
}
