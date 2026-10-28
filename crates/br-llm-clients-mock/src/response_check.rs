use br_llm_clients_core::{Capabilities, ClientError, ClientRequest, OutputMode};
use br_llm_messages::{ContentBlock, ModelResponse, ResponseEnding};

#[derive(Default)]
struct Counts {
    text: usize,
    structured: usize,
    tool_calls: usize,
    provider: usize,
}

impl Counts {
    fn has_content(&self) -> bool {
        self.text > 0 || self.structured > 0 || self.tool_calls > 0 || self.provider > 0
    }
}

fn count(response: &ModelResponse) -> Counts {
    let mut counts = Counts::default();
    for block in response.blocks() {
        match block {
            ContentBlock::Text(_) => counts.text = counts.text.saturating_add(1),
            ContentBlock::StructuredContent(_) => {
                counts.structured = counts.structured.saturating_add(1)
            }
            ContentBlock::RuntimeToolCall(_) => {
                counts.tool_calls = counts.tool_calls.saturating_add(1)
            }
            ContentBlock::ProviderToolCall(_)
            | ContentBlock::ProviderToolResult(_)
            | ContentBlock::ProviderToolExecutionRecord(_) => {
                counts.provider = counts.provider.saturating_add(1)
            }
            _ => {}
        }
    }
    counts
}

fn blocked(ending: &ResponseEnding) -> bool {
    *ending == ResponseEnding::filtering() || *ending == ResponseEnding::refusal()
}

pub(crate) fn check(
    capabilities: &Capabilities,
    request: &ClientRequest,
    response: &ModelResponse,
) -> Result<(), ClientError> {
    let counts = count(response);
    let ending = response.ending();
    if !counts.has_content() {
        return Err(if blocked(ending) {
            ClientError::ContentBlocked
        } else {
            ClientError::ReplyUnusable {
                ending: ending.clone(),
            }
        });
    }
    let structured = matches!(request.output(), OutputMode::Structured(_));
    if structured && counts.structured == 0 && counts.tool_calls == 0 && blocked(ending) {
        return Err(ClientError::ContentBlocked);
    }
    if structured {
        check_structured_shape(capabilities, request, &counts)
    } else if counts.structured > 0 {
        Err(malformed(
            "a structured block in a reply to a plain request",
        ))
    } else {
        Ok(())
    }
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
    Err(malformed("the response does not fit a structured request"))
}

fn malformed(detail: &str) -> ClientError {
    ClientError::MalformedResponse {
        detail: detail.to_owned(),
    }
}
