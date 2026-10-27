use br_llm_messages::{PlainBlock, StructuredBlock};

use super::escape;
use super::refusal::invalid;
use super::types::{RenderedPart, RenderedResultPart};
use crate::client_error::ClientError;

pub(super) fn part(block: &PlainBlock, framed: bool) -> Result<RenderedPart, ClientError> {
    Ok(match block {
        PlainBlock::Text(text) => RenderedPart::Text(shaped(text.text().as_str(), framed)),
        PlainBlock::Image(image) => RenderedPart::Image(image.clone()),
        PlainBlock::StructuredContent(structured) => {
            RenderedPart::Text(shaped(&structured_json(structured)?, framed))
        }
        other => return Err(unknown_kind(other)),
    })
}

pub(super) fn structured_json(structured: &StructuredBlock) -> Result<String, ClientError> {
    structured.value().to_json_string().map_err(|_| {
        invalid(format!(
            "structured block {} cannot be written as JSON",
            structured.id()
        ))
    })
}

fn unknown_kind(block: &PlainBlock) -> ClientError {
    invalid(format!(
        "unknown content block kind {}",
        block.kind().as_str()
    ))
}

pub(super) fn plain_parts(content: &[PlainBlock]) -> Result<Vec<RenderedPart>, ClientError> {
    content.iter().map(|block| part(block, false)).collect()
}

pub(super) fn result_parts(content: &[PlainBlock]) -> Result<Vec<RenderedResultPart>, ClientError> {
    content
        .iter()
        .map(|block| {
            Ok(match block {
                PlainBlock::Text(text) => RenderedResultPart::Text(text.text().as_str().to_owned()),
                PlainBlock::Image(image) => RenderedResultPart::Image(image.clone()),
                PlainBlock::StructuredContent(structured) => {
                    RenderedResultPart::Text(structured_json(structured)?)
                }
                other => return Err(unknown_kind(other)),
            })
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
