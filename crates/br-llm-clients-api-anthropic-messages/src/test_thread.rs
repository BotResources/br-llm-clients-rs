use std::sync::atomic::{AtomicU64, Ordering};

use br_llm_messages::{
    AITurn, AgentId, Author, Base64Data, BlockId, ContentBlock, Conversation, ConversationId,
    EntryId, ExternalId, HumanId, HumanMessage, ImageBlock, ImageSource, IncomingEntry, InputTurn,
    ItemRef, MediaType, ModelResponse, OpaqueJson, Perspective, PlainBlock, ProjectedThread,
    RedactedThinkingBlock, ReplayFormat, ReplayMaterial, ReplayPayload, ResponseEnding,
    ResponseMetadata, RuntimeToolCall, RuntimeToolResult, RuntimeToolResults, TextBlock,
    ThinkingBlock, ToolName, Turn, TurnEntry, TurnId, Uuid,
};
use serde_json::Value;

use crate::replay_formats::{REDACTED_DATA_FORMAT, SIGNATURE_FORMAT};

static NEXT: AtomicU64 = AtomicU64::new(0x10_0000);

pub(crate) fn uuid_of(n: u64) -> Uuid {
    Uuid::from_u128(0x0190_0000_0000_7000_8000_0000_0000_0000 | u128::from(n))
}

fn fresh() -> Uuid {
    uuid_of(NEXT.fetch_add(1, Ordering::Relaxed))
}

pub(crate) fn block_id() -> BlockId {
    BlockId::from_uuid(fresh()).expect("a block id")
}

fn entry_id() -> EntryId {
    EntryId::from_uuid(fresh()).expect("an entry id")
}

pub(crate) fn reply_turn() -> TurnId {
    TurnId::from_uuid(uuid_of(0x1000)).expect("a turn id")
}

pub(crate) fn reply_entry() -> EntryId {
    EntryId::from_uuid(uuid_of(0x2000)).expect("an entry id")
}

fn agent() -> AgentId {
    AgentId::from_uuid(uuid_of(0x3000)).expect("an agent id")
}

pub(crate) fn project(turns: Vec<Turn>) -> ProjectedThread {
    let id = ConversationId::from_uuid(uuid_of(0x3200)).expect("a conversation id");
    let conversation = Conversation::from_parts(id, turns).expect("a conversation");
    Perspective::of(agent()).project(&conversation)
}

pub(crate) fn plain_text(value: &str) -> PlainBlock {
    PlainBlock::Text(TextBlock::new(block_id(), value).expect("a text"))
}

pub(crate) fn image_block(media_type: MediaType, data: &str) -> ImageBlock {
    let data = Base64Data::new(data).expect("base64 data");
    ImageBlock::new(block_id(), ImageSource::inline(media_type, data))
}

pub(crate) fn plain_image(media_type: MediaType, data: &str) -> PlainBlock {
    PlainBlock::Image(image_block(media_type, data))
}

pub(crate) fn human_with(content: Vec<PlainBlock>) -> Turn {
    let human = HumanId::from_uuid(uuid_of(0x3100)).expect("a human id");
    let message = HumanMessage::new(entry_id(), human, content).expect("a message");
    let turn = TurnId::from_uuid(fresh()).expect("a turn id");
    Turn::InputTurn(
        InputTurn::new(turn, vec![IncomingEntry::from(message)]).expect("an input turn"),
    )
}

pub(crate) fn user(value: &str) -> Turn {
    human_with(vec![plain_text(value)])
}

fn ai_turn(id: TurnId, entries: Vec<TurnEntry>) -> Turn {
    let author = Author::new("target").expect("an author");
    Turn::AITurn(AITurn::from_parts(id, agent(), author, entries).expect("an ai turn"))
}

pub(crate) fn earlier(entries: Vec<TurnEntry>) -> Turn {
    ai_turn(TurnId::from_uuid(fresh()).expect("a turn id"), entries)
}

pub(crate) fn in_progress(entries: Vec<TurnEntry>) -> Turn {
    ai_turn(reply_turn(), entries)
}

pub(crate) fn response(blocks: Vec<ContentBlock>, ending: ResponseEnding) -> TurnEntry {
    TurnEntry::ModelResponse(
        ModelResponse::new(entry_id(), blocks, ending, ResponseMetadata::new())
            .expect("a response"),
    )
}

pub(crate) fn answer(blocks: Vec<ContentBlock>) -> TurnEntry {
    response(blocks, ResponseEnding::finished())
}

pub(crate) fn calling(blocks: Vec<ContentBlock>) -> TurnEntry {
    response(blocks, ResponseEnding::tool_use())
}

pub(crate) fn assistant_text(value: &str) -> Turn {
    earlier(vec![answer(vec![text(value)])])
}

pub(crate) fn text(value: &str) -> ContentBlock {
    ContentBlock::Text(TextBlock::new(block_id(), value).expect("a text"))
}

pub(crate) fn material(format: &str, value: &str) -> ReplayMaterial {
    ReplayMaterial::new(
        ReplayFormat::new(format).expect("a format"),
        ReplayPayload::text(value),
    )
}

pub(crate) fn signature(value: &str) -> ReplayMaterial {
    material(SIGNATURE_FORMAT, value)
}

pub(crate) fn redacted_data(value: &str) -> ReplayMaterial {
    material(REDACTED_DATA_FORMAT, value)
}

pub(crate) fn thinking(value: &str, replay: Option<ReplayMaterial>) -> ContentBlock {
    let block = match replay {
        Some(replay) if value.is_empty() => ThinkingBlock::replay_only(block_id(), replay),
        Some(replay) => ThinkingBlock::new(block_id(), value)
            .expect("a thinking block")
            .with_replay(replay),
        None => ThinkingBlock::new(block_id(), value).expect("a thinking block"),
    };
    ContentBlock::Thinking(block)
}

pub(crate) fn redacted(replay: ReplayMaterial) -> ContentBlock {
    ContentBlock::RedactedThinking(RedactedThinkingBlock::new(block_id(), replay))
}

pub(crate) fn call(id: &str, name: &str, arguments: Value) -> RuntimeToolCall {
    let arguments = OpaqueJson::parse(&arguments.to_string()).expect("json arguments");
    RuntimeToolCall::new(
        block_id(),
        ToolName::new(name).expect("a tool name"),
        arguments,
    )
    .expect("a tool call")
    .with_external_id(ExternalId::new(id).expect("an external id"))
}

pub(crate) fn call_block(call: &RuntimeToolCall) -> ContentBlock {
    ContentBlock::RuntimeToolCall(call.clone())
}

pub(crate) fn result_of(
    call: &RuntimeToolCall,
    content: Vec<PlainBlock>,
    is_error: bool,
) -> RuntimeToolResult {
    RuntimeToolResult::new(
        block_id(),
        call.id(),
        call.name().clone(),
        is_error,
        content,
    )
    .expect("a tool result")
}

pub(crate) fn results(results: Vec<RuntimeToolResult>) -> TurnEntry {
    TurnEntry::RuntimeToolResults(RuntimeToolResults::new(entry_id(), results).expect("results"))
}

pub(crate) fn one_call_turn(
    call: &RuntimeToolCall,
    content: Vec<PlainBlock>,
    is_error: bool,
) -> Turn {
    in_progress(vec![
        calling(vec![call_block(call)]),
        results(vec![result_of(call, content, is_error)]),
    ])
}

pub(crate) fn item_ref(value: &str) -> ItemRef {
    ItemRef::new(value).expect("an item reference")
}

pub(crate) fn texts(values: &[&str]) -> Vec<PlainBlock> {
    values.iter().map(|value| plain_text(value)).collect()
}

pub(crate) fn in_progress_with(block: ContentBlock) -> Vec<Turn> {
    let call = call("t1", "lookup", serde_json::json!({}));
    vec![
        user("go"),
        in_progress(vec![
            calling(vec![block, call_block(&call)]),
            results(vec![result_of(&call, texts(&["r"]), false)]),
        ]),
    ]
}
