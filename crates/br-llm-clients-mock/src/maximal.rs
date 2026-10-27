use br_llm_messages::{
    BlockId, Citation, CitationId, CitationSource, ContentBlock, ExecutionStatus, ExternalId,
    ImageBlock, ImageSource, ItemRef, MediaType, ModelResponse, ObservedAt, OpaqueJson, PlainBlock,
    ProviderOperation, ProviderToolCall, ProviderToolExecutionRecord, ProviderToolResult,
    RedactedThinkingBlock, ReplayFormat, ReplayMaterial, ReplayPayload, ResponseEnding,
    ResponseMetadata, RuntimeToolCall, TextBlock, TextRange, ThinkingBlock, ToolName,
};

use crate::fixtures::block_id;
use crate::test_support::{model, usage};

fn observed() -> ObservedAt {
    ObservedAt::new("2026-10-27T10:00:00Z").unwrap()
}

fn replay() -> ReplayMaterial {
    ReplayMaterial::new(
        ReplayFormat::new("fmt").unwrap(),
        ReplayPayload::text("opaque"),
    )
}

fn cited_text(value: &str, citations: usize) -> TextBlock {
    let citations = (0..citations)
        .map(|index| {
            let id = CitationId::from_uuid(block_id().as_uuid()).unwrap();
            let source = CitationSource::new(format!("https://example.test/{index}"))
                .unwrap()
                .with_title("title");
            Citation::new(id, source).with_range(TextRange::new(0, 2).unwrap())
        })
        .collect();
    TextBlock::new(block_id(), value)
        .unwrap()
        .with_citations(citations)
        .unwrap()
}

fn image() -> ImageBlock {
    ImageBlock::new(
        block_id(),
        ImageSource::inline(
            MediaType::Png,
            br_llm_messages::Base64Data::new("aGVsbG8=").unwrap(),
        ),
    )
    .with_observed_at(observed())
}

pub(crate) struct Maximal {
    pub(crate) response: ModelResponse,
    pub(crate) draws: usize,
    pub(crate) provider_call: BlockId,
}

pub(crate) fn maximal() -> Maximal {
    let operation = ProviderOperation::new("web_search").unwrap();
    let provider_call = ProviderToolCall::new(block_id())
        .with_external_id(ExternalId::new("pc_1").unwrap())
        .with_item_ref(ItemRef::new("item_pc").unwrap())
        .with_operation(operation.clone())
        .with_arguments(OpaqueJson::parse(r#"{"q":"x"}"#).unwrap())
        .unwrap()
        .with_replay(replay())
        .with_observed_at(observed());
    let call_id = provider_call.id();
    let result = ProviderToolResult::new(
        block_id(),
        call_id,
        vec![
            PlainBlock::try_from(cited_text("found", 1)).unwrap(),
            PlainBlock::Image(image()),
        ],
    )
    .unwrap()
    .with_external_id(ExternalId::new("pr_1").unwrap())
    .with_observed_at(observed());
    let record = ProviderToolExecutionRecord::new(block_id())
        .with_external_id(ExternalId::new("rec_1").unwrap())
        .with_operation(operation)
        .with_status(ExecutionStatus::Completed)
        .with_content(vec![PlainBlock::try_from(cited_text("record", 1)).unwrap()])
        .unwrap()
        .with_replay(replay())
        .with_observed_at(observed());
    let blocks = vec![
        ThinkingBlock::new(block_id(), "hm")
            .unwrap()
            .with_replay(replay())
            .with_observed_at(observed())
            .into(),
        RedactedThinkingBlock::new(block_id(), replay())
            .with_observed_at(observed())
            .into(),
        cited_text("cited", 2)
            .with_item_ref(ItemRef::new("item_t").unwrap())
            .with_replay(replay())
            .with_observed_at(observed())
            .into(),
        ContentBlock::Image(image()),
        RuntimeToolCall::new(
            block_id(),
            ToolName::new("search").unwrap(),
            OpaqueJson::parse(r#"{"q":1}"#).unwrap(),
        )
        .unwrap()
        .with_external_id(ExternalId::new("rc_1").unwrap())
        .with_item_ref(ItemRef::new("item_rc").unwrap())
        .with_replay(replay())
        .with_observed_at(observed())
        .into(),
        provider_call.into(),
        result.into(),
        record.into(),
    ];
    let metadata = ResponseMetadata::new()
        .with_usage(usage(5, 7))
        .with_model(model("answering"))
        .with_provider_response_id(ExternalId::new("resp_1").unwrap())
        .with_observed_at(observed());
    let response = ModelResponse::new(
        crate::fixtures::entry_id(),
        blocks,
        ResponseEnding::tool_use(),
        metadata,
    )
    .unwrap();
    Maximal {
        response,
        draws: 15,
        provider_call: call_id,
    }
}
