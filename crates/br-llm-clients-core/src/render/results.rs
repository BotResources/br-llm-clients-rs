use br_llm_messages::{BlockId, RuntimeToolResults};

use super::calls::CallTable;
use super::content;
use super::merge::Item;
use super::refusal::invalid;
use super::types::{RenderedPart, RenderedResultPart};
use crate::client_error::ClientError;

pub(super) const UNRESOLVED_TEXT: &str =
    "No result was recorded for this tool call because its turn ended before a result came.";

pub(super) fn delivered(
    entry: &RuntimeToolResults,
    calls: &CallTable,
) -> Result<Item, ClientError> {
    let mut results = Vec::with_capacity(entry.results().len());
    for result in entry.results() {
        let info = calls.get(result.call()).ok_or_else(|| {
            invalid(format!(
                "tool result answers unknown call {}",
                result.call()
            ))
        })?;
        results.push((
            info.index,
            RenderedPart::ToolResult {
                wire_id: info.wire_id.clone(),
                tool_name: result.tool_name().clone(),
                is_error: result.is_error(),
                parts: content::result_parts(result.content())?,
            },
        ));
    }
    Ok(Item::results(results))
}

pub(super) fn unresolved(outstanding: &[BlockId], calls: &CallTable) -> Result<Item, ClientError> {
    let mut results = Vec::with_capacity(outstanding.len());
    for call in outstanding {
        let info = calls
            .get(*call)
            .ok_or_else(|| invalid(format!("tool call {call} has no wire identity")))?;
        results.push((
            info.index,
            RenderedPart::ToolResult {
                wire_id: info.wire_id.clone(),
                tool_name: info.tool_name.clone(),
                is_error: true,
                parts: vec![RenderedResultPart::Text(UNRESOLVED_TEXT.to_owned())],
            },
        ));
    }
    Ok(Item::results(results))
}
