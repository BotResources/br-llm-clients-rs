use br_llm_messages::{AssistantBlock, MessageError, Step, ToolCall, ToolCallId};

#[allow(
    clippy::expect_used,
    reason = "the text holds a dash, and ToolCallId refuses only an empty text"
)]
pub fn tool_call_id(prefix: &str, sequence: usize, index: Option<usize>) -> ToolCallId {
    let text = match index {
        Some(index) => format!("{prefix}-{sequence}-{index}"),
        None => format!("{prefix}-{sequence}"),
    };
    ToolCallId::new(text).expect("an id that holds a dash is not empty")
}

pub(crate) fn rename_tool_calls(
    step: &Step,
    prefix: &str,
    sequence: usize,
) -> Result<Step, MessageError> {
    let mut position = 0;
    let content = step
        .content()
        .iter()
        .map(|block| match block {
            AssistantBlock::ToolCall(call) => {
                let renamed = ToolCall {
                    id: tool_call_id(prefix, sequence, Some(position)),
                    name: call.name.clone(),
                    arguments: call.arguments.clone(),
                };
                position += 1;
                AssistantBlock::ToolCall(renamed)
            }
            other => other.clone(),
        })
        .collect();
    Step::new(
        content,
        step.stop_reason().clone(),
        step.usage().cloned(),
        step.model().cloned(),
    )
}
