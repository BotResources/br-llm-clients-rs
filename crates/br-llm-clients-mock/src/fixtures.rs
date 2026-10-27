use std::cell::Cell;

use br_llm_messages::{
    AITurn, AgentId, Author, Base64Data, BlockId, ContentBlock, Conversation, ConversationId,
    EntryId, HumanId, HumanMessage, ImageBlock, ImageSource, IncomingEntry, InputTurn, MediaType,
    ModelResponse, OpaqueJson, Perspective, PlainBlock, ProjectedThread, ResponseEnding,
    ResponseMetadata, RuntimeToolCall, RuntimeToolResult, RuntimeToolResults, TextBlock, ToolName,
    Turn, TurnEntry, TurnId, Uuid,
};

use crate::sequential_id_source::identity_uuid;

thread_local! {
    static NEXT: Cell<u64> = const { Cell::new(1000) };
}

fn next_uuid() -> Uuid {
    identity_uuid(NEXT.with(|next| {
        let number = next.get();
        next.set(number + 1);
        number
    }))
}

pub(crate) fn with_fixed_ids<T>(start: u64, build: impl FnOnce() -> T) -> T {
    let saved = NEXT.with(|next| next.replace(start));
    let built = build();
    NEXT.with(|next| next.set(saved));
    built
}

pub(crate) fn turn_id() -> TurnId {
    TurnId::from_uuid(next_uuid()).unwrap()
}

pub(crate) fn entry_id() -> EntryId {
    EntryId::from_uuid(next_uuid()).unwrap()
}

pub(crate) fn block_id() -> BlockId {
    BlockId::from_uuid(next_uuid()).unwrap()
}

pub(crate) fn agent_id() -> AgentId {
    AgentId::from_uuid(next_uuid()).unwrap()
}

pub(crate) fn text_block(value: &str) -> PlainBlock {
    PlainBlock::Text(TextBlock::new(block_id(), value).unwrap())
}

pub(crate) fn plain_image(media_type: MediaType) -> PlainBlock {
    PlainBlock::Image(ImageBlock::new(
        block_id(),
        ImageSource::inline(media_type, Base64Data::new("aGVsbG8=").unwrap()),
    ))
}

pub(crate) fn human(content: Vec<PlainBlock>) -> IncomingEntry {
    let human = HumanId::from_uuid(next_uuid()).unwrap();
    HumanMessage::new(entry_id(), human, content)
        .unwrap()
        .into()
}

pub(crate) fn human_turn(content: Vec<PlainBlock>) -> Turn {
    Turn::InputTurn(InputTurn::new(turn_id(), vec![human(content)]).unwrap())
}

pub(crate) fn unanswered_call_turn(agent: AgentId) -> Turn {
    let call = RuntimeToolCall::new(
        block_id(),
        ToolName::new("search").unwrap(),
        OpaqueJson::parse("{}").unwrap(),
    )
    .unwrap();
    let response = ModelResponse::new(
        entry_id(),
        vec![ContentBlock::RuntimeToolCall(call)],
        ResponseEnding::tool_use(),
        ResponseMetadata::new(),
    )
    .unwrap();
    let author = Author::new("target").unwrap();
    Turn::AITurn(
        AITurn::from_parts(
            turn_id(),
            agent,
            author,
            vec![TurnEntry::ModelResponse(response)],
        )
        .unwrap(),
    )
}

pub(crate) fn project(agent: AgentId, turns: Vec<Turn>) -> ProjectedThread {
    let conversation = ConversationId::from_uuid(next_uuid()).unwrap();
    let conversation = Conversation::from_parts(conversation, turns).unwrap();
    Perspective::of(agent).project(&conversation)
}

pub(crate) fn thread_of(turns: Vec<Turn>) -> ProjectedThread {
    project(agent_id(), turns)
}

pub(crate) fn answered_turn(agent: AgentId, value: &str) -> Turn {
    let block = TextBlock::new(block_id(), value).unwrap();
    let response = ModelResponse::new(
        entry_id(),
        vec![ContentBlock::Text(block)],
        ResponseEnding::finished(),
        ResponseMetadata::new(),
    )
    .unwrap();
    let author = Author::new("target").unwrap();
    Turn::AITurn(
        AITurn::from_parts(
            turn_id(),
            agent,
            author,
            vec![TurnEntry::ModelResponse(response)],
        )
        .unwrap(),
    )
}

pub(crate) fn tool_exchange_turn(agent: AgentId) -> Turn {
    let call_id = block_id();
    let name = ToolName::new("search").unwrap();
    let call =
        RuntimeToolCall::new(call_id, name.clone(), OpaqueJson::parse("{}").unwrap()).unwrap();
    let response = ModelResponse::new(
        entry_id(),
        vec![ContentBlock::RuntimeToolCall(call)],
        ResponseEnding::tool_use(),
        ResponseMetadata::new(),
    )
    .unwrap();
    let result =
        RuntimeToolResult::new(block_id(), call_id, name, false, vec![text_block("found")])
            .unwrap();
    let results = RuntimeToolResults::new(entry_id(), vec![result]).unwrap();
    let answer = answered_entry("done");
    let author = Author::new("target").unwrap();
    Turn::AITurn(
        AITurn::from_parts(
            turn_id(),
            agent,
            author,
            vec![
                TurnEntry::ModelResponse(response),
                TurnEntry::RuntimeToolResults(results),
                TurnEntry::ModelResponse(answer),
            ],
        )
        .unwrap(),
    )
}

fn answered_entry(value: &str) -> ModelResponse {
    let block = TextBlock::new(block_id(), value).unwrap();
    ModelResponse::new(
        entry_id(),
        vec![ContentBlock::Text(block)],
        ResponseEnding::finished(),
        ResponseMetadata::new(),
    )
    .unwrap()
}
