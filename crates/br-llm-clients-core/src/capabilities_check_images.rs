#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::panic,
    clippy::wildcard_enum_match_arm
)]

use br_llm_messages::{ImageMime, WireMessage, WireUserBlock};

use crate::capabilities::Capabilities;
use crate::client_error::ClientError;
use crate::client_request::ClientRequest;

pub(crate) fn check_images(
    capabilities: &Capabilities,
    request: &ClientRequest,
) -> Result<(), ClientError> {
    let mut mimes = user_image_mimes(request).peekable();
    if mimes.peek().is_none() {
        return Ok(());
    }
    if !capabilities.images() {
        return Err(ClientError::ImageNotSupported);
    }
    match mimes.find(|mime| !capabilities.image_media_types().accepts(mime)) {
        Some(media_type) => Err(ClientError::ImageMediaTypeNotSupported { media_type }),
        None => Ok(()),
    }
}

fn user_image_mimes(request: &ClientRequest) -> impl Iterator<Item = ImageMime> + '_ {
    request
        .messages()
        .iter()
        .flat_map(|message| match message {
            WireMessage::User { content } => content.as_slice(),
            WireMessage::Assistant { .. } | WireMessage::Relay { .. } => &[],
        })
        .filter_map(|block| match block {
            WireUserBlock::Image(image) => Some(image.mime),
            WireUserBlock::Text { .. } | WireUserBlock::ToolResult(_) => None,
        })
}
