use br_llm_clients_core::{ClientError, ClientRequest, OutputMode};
use br_llm_messages::{
    ContentBlock, ImageBlock, ImageSource, MediaType, ModelResponse, OpaqueJson, ProviderToolCall,
    ResponseEnding, ResponseMetadata, StructuredBlock, ThinkingBlock,
};

use crate::fixtures::{block_id, text_block};
use crate::test_support::*;
use crate::{MockClient, Reply};

pub(crate) fn structured_content(json: &str) -> ContentBlock {
    ContentBlock::StructuredContent(
        StructuredBlock::new(block_id(), OpaqueJson::parse(json).unwrap()).unwrap(),
    )
}

pub(crate) fn thinking_content() -> ContentBlock {
    ContentBlock::Thinking(ThinkingBlock::new(block_id(), "hm").unwrap())
}

pub(crate) fn image_content() -> ContentBlock {
    ContentBlock::Image(ImageBlock::new(
        block_id(),
        ImageSource::inline(
            MediaType::Png,
            br_llm_messages::Base64Data::new("aGVsbG8=").unwrap(),
        ),
    ))
}

pub(crate) fn provider_content() -> ContentBlock {
    ContentBlock::ProviderToolCall(ProviderToolCall::new(block_id()))
}

pub(crate) fn response_ending(blocks: Vec<ContentBlock>, ending: ResponseEnding) -> ModelResponse {
    response_of(blocks, ending, ResponseMetadata::new())
}

pub(crate) fn plain_request() -> ClientRequest {
    request()
}

pub(crate) fn structured_request() -> ClientRequest {
    request_of(vec![text_block("ask")], vec![], shape())
}

pub(crate) fn structured_with_tools_request() -> ClientRequest {
    request_of(vec![text_block("ask")], vec![tool()], shape())
}

pub(crate) fn tools_request() -> ClientRequest {
    request_of(vec![text_block("ask")], vec![tool()], OutputMode::Plain)
}

pub(crate) fn serve(
    request: &ClientRequest,
    response: ModelResponse,
) -> Result<ModelResponse, ClientError> {
    let client: MockClient = mock();
    client.push_reply(Reply::success(response).build().unwrap());
    let (outcome, _) = run(&client, request);
    outcome.result().cloned().map_err(Clone::clone)
}
