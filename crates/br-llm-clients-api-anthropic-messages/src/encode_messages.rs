use br_llm_clients_core::ClientError;
use br_llm_messages::{WireMessage, WireUserBlock};
use serde_json::{Value, json};

use crate::encode_blocks::{assistant_block, invalid, text_block, user_block};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    User,
    Assistant,
}

impl Role {
    fn name(self) -> &'static str {
        match self {
            Role::User => "user",
            Role::Assistant => "assistant",
        }
    }
}

struct Draft {
    role: Role,
    blocks: Vec<Value>,
}

pub(crate) fn encode_messages(messages: &[WireMessage]) -> Result<Vec<Value>, ClientError> {
    let turn_start = last_user_without_result(messages);
    let mut drafts: Vec<Draft> = Vec::new();
    for (position, message) in messages.iter().enumerate() {
        match message {
            WireMessage::User { content } => {
                if content.is_empty() {
                    return Err(invalid("a user message holds no block"));
                }
                drafts.push(Draft {
                    role: Role::User,
                    blocks: user_blocks(content)?,
                });
            }
            WireMessage::Assistant { content } => {
                let in_progress = turn_start.is_none_or(|start| position > start);
                drafts.push(Draft {
                    role: Role::Assistant,
                    blocks: assistant_blocks(content, in_progress)?,
                });
            }
            WireMessage::Relay { content } => {
                let texts = content.iter().map(|text| text_block(text.as_str()));
                match drafts.last_mut() {
                    Some(last) if last.role == Role::User => last.blocks.extend(texts),
                    _ => drafts.push(Draft {
                        role: Role::User,
                        blocks: texts.collect(),
                    }),
                }
            }
        }
    }
    check_drafts(&drafts)?;
    Ok(drafts
        .into_iter()
        .map(|draft| json!({"role": draft.role.name(), "content": draft.blocks}))
        .collect())
}

fn check_drafts(drafts: &[Draft]) -> Result<(), ClientError> {
    let Some(last) = drafts.last() else {
        return Err(invalid("the request holds no message"));
    };
    if last.role == Role::Assistant {
        return Err(invalid("the last message is an assistant message"));
    }
    for draft in drafts {
        if draft.blocks.is_empty() {
            return Err(invalid(match draft.role {
                Role::User => "a user message holds no block",
                Role::Assistant => "an assistant message holds no block that can be sent",
            }));
        }
    }
    Ok(())
}

fn last_user_without_result(messages: &[WireMessage]) -> Option<usize> {
    messages.iter().rposition(|message| match message {
        WireMessage::User { content } => !content
            .iter()
            .any(|block| matches!(block, WireUserBlock::ToolResult(_))),
        WireMessage::Assistant { .. } | WireMessage::Relay { .. } => false,
    })
}

fn user_blocks(content: &[WireUserBlock]) -> Result<Vec<Value>, ClientError> {
    let is_result = |block: &&WireUserBlock| matches!(block, WireUserBlock::ToolResult(_));
    let results = content.iter().filter(is_result);
    let others = content.iter().filter(|block| !is_result(block));
    results.chain(others).map(user_block).collect()
}

fn assistant_blocks(
    content: &[br_llm_messages::AssistantBlock],
    in_progress: bool,
) -> Result<Vec<Value>, ClientError> {
    let mut blocks = Vec::new();
    for block in content {
        blocks.extend(assistant_block(block, in_progress)?);
    }
    Ok(blocks)
}
