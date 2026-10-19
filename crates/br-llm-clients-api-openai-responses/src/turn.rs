use br_llm_messages::{WireMessage, WireUserBlock};

fn holds_tool_result(content: &[WireUserBlock]) -> bool {
    content
        .iter()
        .any(|block| matches!(block, WireUserBlock::ToolResult(_)))
}

pub(crate) fn turn_start(messages: &[WireMessage]) -> Option<usize> {
    messages.iter().rposition(
        |message| matches!(message, WireMessage::User { content } if !holds_tool_result(content)),
    )
}

pub(crate) fn is_in_progress(start: Option<usize>, index: usize) -> bool {
    start.is_some_and(|start| index > start)
}
