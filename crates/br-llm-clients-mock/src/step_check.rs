use std::collections::HashSet;

use br_llm_clients_core::{ClientError, ClientRequest, OutputMode};
use br_llm_messages::{AssistantBlock, Step, StopReason, ToolCallId, WireMessage};

use br_llm_clients_core::Capabilities;

struct Counts {
    text: usize,
    structured: usize,
    tool_calls: usize,
}

impl Counts {
    fn is_empty(&self) -> bool {
        self.text == 0 && self.structured == 0 && self.tool_calls == 0
    }
}

fn count(step: &Step) -> Counts {
    let mut counts = Counts {
        text: 0,
        structured: 0,
        tool_calls: 0,
    };
    for block in step.content() {
        match block {
            AssistantBlock::Text { .. } => counts.text = counts.text.saturating_add(1),
            AssistantBlock::Structured { .. } => {
                counts.structured = counts.structured.saturating_add(1)
            }
            AssistantBlock::ToolCall(_) => counts.tool_calls = counts.tool_calls.saturating_add(1),
            AssistantBlock::Thinking(_) | AssistantBlock::RedactedThinking(_) => {}
            // `AssistantBlock` grows only by an intent change of br-llm-messages: a new
            // minor, so a new 0.x line (0.6) outside the `^0.5.0` requirement. No lawful
            // 0.5.x adds a block, so the five above are every block a `Step` holds here.
            // A block is never left uncounted.
            _ => unreachable!("br-llm-messages 0.5 has no other assistant block"),
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
    step: &Step,
) -> Result<(), ClientError> {
    let counts = count(step);
    let stop_reason = step.stop_reason();
    if counts.is_empty() {
        return Err(if blocked(stop_reason) {
            ClientError::ContentBlocked
        } else {
            ClientError::ReplyUnusable {
                stop_reason: stop_reason.clone(),
            }
        });
    }
    let structured = matches!(request.output(), OutputMode::Structured(_));
    if structured
        && counts.structured.saturating_add(counts.tool_calls) == 0
        && blocked(stop_reason)
    {
        return Err(ClientError::ContentBlocked);
    }
    if structured {
        check_structured_shape(capabilities, request, &counts)?;
    } else if counts.structured > 0 {
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
