use br_llm_messages::{ContentBlock, ModelResponse, RuntimeToolCall};

use super::calls::CallTable;
use super::content;
use super::refusal::invalid;
use super::types::RenderedPart;
use crate::client_error::ClientError;

pub(super) fn parts(
    response: &ModelResponse,
    reply: bool,
    calls: &CallTable,
) -> Result<Vec<RenderedPart>, ClientError> {
    let mut parts = Vec::new();
    for block in response.blocks() {
        match block {
            ContentBlock::Text(text) => {
                if reply && (text.replay().is_some() || text.item_ref().is_some()) {
                    parts.push(RenderedPart::Block(block.clone()));
                } else {
                    parts.push(RenderedPart::Text(text.text().as_str().to_owned()));
                }
            }
            ContentBlock::Image(image) => parts.push(RenderedPart::Image(image.clone())),
            ContentBlock::StructuredContent(structured) => {
                parts.push(RenderedPart::Text(content::structured_json(structured)?));
            }
            ContentBlock::Thinking(_)
            | ContentBlock::RedactedThinking(_)
            | ContentBlock::ProviderToolCall(_)
            | ContentBlock::ProviderToolResult(_)
            | ContentBlock::ProviderToolExecutionRecord(_) => {
                if reply {
                    parts.push(RenderedPart::Block(block.clone()));
                }
            }
            ContentBlock::RuntimeToolCall(call) => {
                let info = calls.get(call.id()).ok_or_else(|| {
                    invalid(format!("tool call {} has no wire identity", call.id()))
                })?;
                let call = if reply {
                    call.clone()
                } else {
                    without_replay(call)?
                };
                parts.push(RenderedPart::ToolCall {
                    call,
                    wire_id: info.wire_id.clone(),
                });
            }
            other => {
                return Err(invalid(format!(
                    "unexpected content block kind {} in a model response",
                    other.kind().as_str()
                )));
            }
        }
    }
    Ok(parts)
}

fn without_replay(call: &RuntimeToolCall) -> Result<RuntimeToolCall, ClientError> {
    let rebuilt = RuntimeToolCall::new(call.id(), call.name().clone(), call.arguments().clone())
        .map_err(|_| invalid(format!("tool call {} cannot be rebuilt", call.id())))?;
    let rebuilt = match call.external_id() {
        Some(external) => rebuilt.with_external_id(external.clone()),
        None => rebuilt,
    };
    Ok(match call.observed_at() {
        Some(observed_at) => rebuilt.with_observed_at(observed_at.clone()),
        None => rebuilt,
    })
}
