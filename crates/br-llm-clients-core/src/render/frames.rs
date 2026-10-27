use br_llm_messages::{Author, PlainBlock};

use super::content;
use super::escape;
use super::types::RenderedPart;
use crate::client_error::ClientError;

pub(super) const AGENT_REQUEST: &str = "agent_request";
pub(super) const AGENT_MAIL: &str = "agent_mail";
pub(super) const AGENT_OUTPUT: &str = "agent_output";
pub(super) const PARTIAL_AGENT_OUTPUT: &str = "partial_agent_output";

pub(super) fn framed(
    name: &str,
    author: &Author,
    content: &[PlainBlock],
) -> Result<Vec<RenderedPart>, ClientError> {
    let mut parts = Vec::with_capacity(content.len().saturating_add(2));
    parts.push(RenderedPart::Text(format!(
        "<{name} author=\"{}\">",
        escape::label(author.as_str())
    )));
    for block in content {
        parts.push(content::part(block, true)?);
    }
    parts.push(RenderedPart::Text(format!("</{name}>")));
    Ok(parts)
}
