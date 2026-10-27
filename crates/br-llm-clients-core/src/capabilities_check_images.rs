#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::panic
)]

use br_llm_messages::{
    ContentBlock, ImageBlock, ImageSource, IncomingEntry, MediaType, PlainBlock, Turn, TurnEntry,
};

use crate::capabilities::Capabilities;
use crate::client_error::ClientError;
use crate::client_request::ClientRequest;

pub(crate) fn check_images(
    capabilities: &Capabilities,
    request: &ClientRequest,
) -> Result<(), ClientError> {
    let images = images_in_order(request);
    if images.is_empty() {
        return Ok(());
    }
    if !capabilities.images() {
        return Err(ClientError::ImageNotSupported);
    }
    match images
        .into_iter()
        .flatten()
        .find(|media_type| !capabilities.image_media_types().accepts(media_type))
    {
        Some(media_type) => Err(ClientError::ImageMediaTypeNotSupported { media_type }),
        None => Ok(()),
    }
}

fn images_in_order(request: &ClientRequest) -> Vec<Option<MediaType>> {
    let mut images = Vec::new();
    for turn in request.history().turns() {
        match turn {
            Turn::InputTurn(input) => {
                for entry in input.entries() {
                    read_incoming(entry, &mut images);
                }
            }
            Turn::AITurn(ai) => {
                let reply = ai.id() == request.reply_turn();
                for entry in ai.entries() {
                    read_turn_entry(entry, reply, &mut images);
                }
            }
            _ => {}
        }
    }
    images
}

fn read_incoming(entry: &IncomingEntry, images: &mut Vec<Option<MediaType>>) {
    let content = match entry {
        IncomingEntry::HumanMessage(message) => message.content(),
        IncomingEntry::AgentRequest(request) => request.content(),
        IncomingEntry::AgentMail(mail) => mail.content(),
        IncomingEntry::AgentOutput(output) => output.content(),
        IncomingEntry::PartialAgentOutput(output) => output.content(),
        _ => return,
    };
    read_plain(content, images);
}

fn read_turn_entry(entry: &TurnEntry, reply: bool, images: &mut Vec<Option<MediaType>>) {
    match entry {
        TurnEntry::ModelResponse(response) => {
            for block in response.blocks() {
                read_response_block(block, reply, images);
            }
        }
        TurnEntry::RuntimeToolResults(results) => {
            for result in results.results() {
                read_plain(result.content(), images);
            }
        }
        _ => {}
    }
}

fn read_response_block(block: &ContentBlock, reply: bool, images: &mut Vec<Option<MediaType>>) {
    match block {
        ContentBlock::Image(image) => images.push(media_type_of(image)),
        ContentBlock::ProviderToolResult(result) if reply => read_plain(result.content(), images),
        ContentBlock::ProviderToolExecutionRecord(record) if reply => {
            read_plain(record.content(), images);
        }
        _ => {}
    }
}

fn read_plain(content: &[PlainBlock], images: &mut Vec<Option<MediaType>>) {
    for block in content {
        if let PlainBlock::Image(image) = block {
            images.push(media_type_of(image));
        }
    }
}

fn media_type_of(image: &ImageBlock) -> Option<MediaType> {
    match image.source() {
        ImageSource::Inline { media_type, .. } => Some(*media_type),
        _ => None,
    }
}
