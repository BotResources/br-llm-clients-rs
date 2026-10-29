use std::sync::atomic::{AtomicU64, Ordering};

use br_llm_messages::{
    AITurn, AgentId, Author, Base64Data, BlockId, ClosureReason, ContentBlock, Conversation,
    ConversationId, EntryId, HumanId, HumanMessage, ImageBlock, ImageSource, IncomingEntry,
    InputTurn, MediaType, ModelResponse, OpaqueJson, Perspective, PlainBlock, ProjectedThread,
    RedactedThinkingBlock, ReplayFormat, ReplayMaterial, ReplayPayload, ResponseEnding,
    ResponseMetadata, RuntimeNotice, RuntimeToolCall, RuntimeToolResult, RuntimeToolResults,
    TextBlock, ThinkingBlock, ToolName, Turn, TurnClosure, TurnEntry, TurnId, Uuid,
};

use crate::envelope::REPLAY_FORMAT;

static NEXT: AtomicU64 = AtomicU64::new(0x10_0000);

pub(crate) fn uuid_of(n: u64) -> Uuid {
    Uuid::from_u128(0x0190_0000_0000_7000_8000_0000_0000_0000 | u128::from(n))
}

fn fresh() -> Uuid {
    uuid_of(NEXT.fetch_add(1, Ordering::SeqCst))
}

pub(crate) fn fresh_block() -> BlockId {
    BlockId::from_uuid(fresh()).expect("block id")
}

fn fresh_entry() -> EntryId {
    EntryId::from_uuid(fresh()).expect("entry id")
}

pub(crate) fn reply_turn() -> TurnId {
    TurnId::from_uuid(uuid_of(0x1000)).expect("turn id")
}

pub(crate) fn reply_entry() -> EntryId {
    EntryId::from_uuid(uuid_of(0x2000)).expect("entry id")
}

fn agent() -> AgentId {
    AgentId::from_uuid(uuid_of(0x3000)).expect("agent")
}

pub(crate) fn project(turns: Vec<Turn>) -> ProjectedThread {
    let id = ConversationId::from_uuid(uuid_of(0x3200)).expect("conversation");
    let conversation = Conversation::from_parts(id, turns).expect("conversation");
    Perspective::of(agent()).project(&conversation)
}

pub(crate) fn plain_text(value: &str) -> PlainBlock {
    PlainBlock::Text(TextBlock::new(fresh_block(), value).expect("text"))
}

pub(crate) fn image(media_type: MediaType, data: &str) -> ImageBlock {
    let data = Base64Data::new(data).expect("base64");
    ImageBlock::new(fresh_block(), ImageSource::inline(media_type, data))
}

pub(crate) fn plain_image(media_type: MediaType, data: &str) -> PlainBlock {
    PlainBlock::Image(image(media_type, data))
}

pub(crate) fn human(content: Vec<PlainBlock>) -> Turn {
    let id = HumanId::from_uuid(uuid_of(0x3100)).expect("human");
    let message = HumanMessage::new(fresh_entry(), id, content).expect("message");
    let turn = TurnId::from_uuid(fresh()).expect("turn");
    Turn::InputTurn(InputTurn::new(turn, vec![IncomingEntry::from(message)]).expect("input"))
}

pub(crate) fn say(value: &str) -> Turn {
    human(vec![plain_text(value)])
}

pub(crate) fn text(value: &str) -> ContentBlock {
    ContentBlock::Text(TextBlock::new(fresh_block(), value).expect("text"))
}

pub(crate) fn json(text: &str) -> OpaqueJson {
    OpaqueJson::parse(text).expect("json")
}

pub(crate) fn call(external: Option<&str>, name: &str, arguments: &str) -> RuntimeToolCall {
    let name = ToolName::new(name).expect("name");
    let call = RuntimeToolCall::new(fresh_block(), name, json(arguments)).expect("call");
    match external {
        Some(id) => call.with_external_id(br_llm_messages::ExternalId::new(id).expect("external")),
        None => call,
    }
}

pub(crate) fn result_of(
    call: &RuntimeToolCall,
    content: Vec<PlainBlock>,
    is_error: bool,
) -> RuntimeToolResult {
    RuntimeToolResult::new(
        fresh_block(),
        call.id(),
        call.name().clone(),
        is_error,
        content,
    )
    .expect("result")
}

pub(crate) fn replay(format: &str, payload: ReplayPayload) -> ReplayMaterial {
    ReplayMaterial::new(ReplayFormat::new(format).expect("format"), payload)
}

pub(crate) fn envelope_text(id: &str, encrypted: &str) -> String {
    format!("{{\"id\":\"{id}\",\"encrypted_content\":\"{encrypted}\"}}")
}

pub(crate) fn envelope_replay(id: &str, encrypted: &str) -> ReplayMaterial {
    replay(
        REPLAY_FORMAT,
        ReplayPayload::text(envelope_text(id, encrypted)),
    )
}

pub(crate) fn thinking(value: &str, replay: Option<ReplayMaterial>) -> ContentBlock {
    let block = ThinkingBlock::new(fresh_block(), value).expect("thinking");
    ContentBlock::Thinking(match replay {
        Some(replay) => block.with_replay(replay),
        None => block,
    })
}

pub(crate) fn redacted(data: &str) -> ContentBlock {
    let material = replay("other-library/redacted", ReplayPayload::text(data));
    ContentBlock::RedactedThinking(RedactedThinkingBlock::new(fresh_block(), material))
}

pub(crate) fn response(blocks: Vec<ContentBlock>) -> TurnEntry {
    let calls = blocks
        .iter()
        .any(|block| matches!(block, ContentBlock::RuntimeToolCall(_)));
    let ending = if calls {
        ResponseEnding::tool_use()
    } else {
        ResponseEnding::finished()
    };
    let response = ModelResponse::new(fresh_entry(), blocks, ending, ResponseMetadata::new())
        .expect("response");
    TurnEntry::ModelResponse(response)
}

pub(crate) fn results(list: Vec<RuntimeToolResult>) -> TurnEntry {
    TurnEntry::RuntimeToolResults(RuntimeToolResults::new(fresh_entry(), list).expect("results"))
}

pub(crate) fn notice(value: &str) -> TurnEntry {
    TurnEntry::RuntimeNotice(RuntimeNotice::new(fresh_entry(), value).expect("notice"))
}

pub(crate) fn closed() -> TurnEntry {
    TurnEntry::TurnClosure(TurnClosure::new(
        fresh_entry(),
        ClosureReason::interrupted(),
    ))
}

fn ai_turn(id: TurnId, entries: Vec<TurnEntry>) -> Turn {
    let author = Author::new("target").expect("author");
    Turn::AITurn(AITurn::from_parts(id, agent(), author, entries).expect("ai turn"))
}

pub(crate) fn reply(entries: Vec<TurnEntry>) -> Turn {
    ai_turn(reply_turn(), entries)
}

pub(crate) fn earlier(entries: Vec<TurnEntry>) -> Turn {
    ai_turn(TurnId::from_uuid(fresh()).expect("turn"), entries)
}
