use std::collections::HashSet;

use br_llm_clients_core::{ClientError, ClientRequest};
use br_llm_messages::{AssistantBlock, Step, StopReason, ToolCallId, WireMessage};

use crate::capabilities::Capabilities;
use crate::strictness::ModeClass;

struct Counts {
    text: usize,
    structured: usize,
    tool_calls: usize,
}

fn count(step: &Step) -> Counts {
    let mut counts = Counts {
        text: 0,
        structured: 0,
        tool_calls: 0,
    };
    for block in step.content() {
        match block {
            AssistantBlock::Text { .. } => counts.text += 1,
            AssistantBlock::Structured { .. } => counts.structured += 1,
            AssistantBlock::ToolCall(_) => counts.tool_calls += 1,
            AssistantBlock::Thinking(_) | AssistantBlock::RedactedThinking(_) => {}
        }
    }
    counts
}

fn blocked(stop_reason: &StopReason) -> bool {
    matches!(stop_reason, StopReason::ContentFilter | StopReason::Refusal)
}

pub(crate) fn check(
    capabilities: &Capabilities,
    request: &ClientRequest,
    mode: ModeClass,
    step: &Step,
) -> Result<(), ClientError> {
    let counts = count(step);
    let stop_reason = step.stop_reason();
    if counts.text + counts.structured + counts.tool_calls == 0 {
        return Err(if blocked(stop_reason) {
            ClientError::ContentBlocked
        } else {
            ClientError::ReplyUnusable {
                stop_reason: stop_reason.clone(),
            }
        });
    }
    let structured = mode == ModeClass::Structured;
    if structured && counts.structured + counts.tool_calls == 0 && blocked(stop_reason) {
        return Err(ClientError::ContentBlocked);
    }
    if structured {
        check_structured_shape(capabilities, request, &counts)?;
    } else if counts.structured > 0 && mode == ModeClass::Plain {
        return Err(malformed(
            "a structured block in a reply to a plain request".to_owned(),
        ));
    }
    check_repeated_ids(request, step)
}

fn check_structured_shape(
    capabilities: &Capabilities,
    request: &ClientRequest,
    counts: &Counts,
) -> Result<(), ClientError> {
    let tools_allowed =
        !request.tools().is_empty() && capabilities.accepts_structured_output_with_tools();
    let one_value = counts.structured == 1 && counts.text == 0 && counts.tool_calls == 0;
    let calls_instead = tools_allowed && counts.structured == 0 && counts.tool_calls > 0;
    if one_value || calls_instead {
        return Ok(());
    }
    Err(malformed(
        "the step does not fit a structured request".to_owned(),
    ))
}

fn check_repeated_ids(request: &ClientRequest, step: &Step) -> Result<(), ClientError> {
    let history: HashSet<&ToolCallId> = request
        .messages()
        .iter()
        .filter_map(|message| match message {
            WireMessage::Assistant { content } => Some(content),
            WireMessage::User { .. } | WireMessage::Relay { .. } => None,
        })
        .flatten()
        .filter_map(AssistantBlock::as_tool_call)
        .map(|call| &call.id)
        .collect();
    for call in step.tool_calls() {
        if history.contains(&call.id) {
            return Err(malformed(format!(
                "the mock raised this error: tool call id {} of the reply is already a tool call id of the request",
                call.id
            )));
        }
    }
    Ok(())
}

fn malformed(detail: String) -> ClientError {
    ClientError::MalformedResponse { detail }
}
