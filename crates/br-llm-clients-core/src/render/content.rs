use br_llm_messages::PlainBlock;

use super::escape;
use super::refusal::invalid;
use super::types::{RenderedPart, RenderedResultPart};
use crate::client_error::ClientError;

pub(super) fn part(block: &PlainBlock, framed: bool) -> Result<RenderedPart, ClientError> {
    Ok(match block {
        PlainBlock::Text(text) => RenderedPart::Text(shaped(text.text().as_str(), framed)),
        PlainBlock::Image(image) => RenderedPart::Image(image.clone()),
        PlainBlock::StructuredContent(structured) => {
            let json = structured.value().to_json_string().map_err(|_| {
                invalid(format!(
                    "structured block {} cannot be written as JSON",
                    structured.id()
                ))
            })?;
            RenderedPart::Text(shaped(&json, framed))
        }
        other => {
            return Err(invalid(format!(
                "unknown content block kind {}",
                other.kind().as_str()
            )));
        }
    })
}

pub(super) fn plain_parts(content: &[PlainBlock]) -> Result<Vec<RenderedPart>, ClientError> {
    content.iter().map(|block| part(block, false)).collect()
}

pub(super) fn result_parts(content: &[PlainBlock]) -> Result<Vec<RenderedResultPart>, ClientError> {
    content
        .iter()
        .map(|block| match part(block, false)? {
            RenderedPart::Text(text) => Ok(RenderedResultPart::Text(text)),
            RenderedPart::Image(image) => Ok(RenderedResultPart::Image(image)),
            _ => Err(invalid(format!(
                "unknown content block kind {}",
                block.kind().as_str()
            ))),
        })
        .collect()
}

fn shaped(text: &str, framed: bool) -> String {
    if framed {
        escape::text(text)
    } else {
        text.to_owned()
    }
}
