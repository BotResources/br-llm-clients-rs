use std::sync::atomic::{AtomicU64, Ordering};

use br_llm_messages::{
    AITurn, AgentId, AgentMail, AgentRequest, Author, Base64Data, BlockId, ContentBlock,
    Conversation, ConversationId, DeltaOp, EntryId, HumanId, HumanMessage, ImageBlock, ImageSource,
    IncomingEntry, InputTurn, MediaType, ModelResponse, OpaqueJson, Perspective, PlainBlock,
    ProjectedThread, ProviderToolCall, ProviderToolExecutionRecord, ProviderToolResult, RequestId,
    RequestOrigin, ResponseDelta, ResponseDraft, ResponseEnding, ResponseMetadata, RuntimeToolCall,
    RuntimeToolResult, RuntimeToolResults, TextBlock, ToolName, Turn, TurnEntry, TurnId,
};
use uuid::{Builder, Uuid, Variant, Version};

static NEXT: AtomicU64 = AtomicU64::new(1);

fn next_uuid() -> Uuid {
    let counter = u128::from(NEXT.fetch_add(1, Ordering::Relaxed));
    Builder::from_u128(counter)
        .with_variant(Variant::RFC4122)
        .with_version(Version::SortRand)
        .into_uuid()
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

pub(crate) fn image(media_type: MediaType) -> ImageBlock {
    ImageBlock::new(
        block_id(),
        ImageSource::inline(media_type, Base64Data::new("aGVsbG8=").unwrap()),
    )
}

pub(crate) fn plain_image(media_type: MediaType) -> PlainBlock {
    PlainBlock::Image(image(media_type))
}

pub(crate) fn plain_images(media_types: &[MediaType]) -> Vec<PlainBlock> {
    media_types
        .iter()
        .map(|media| plain_image(*media))
        .collect()
}

pub(crate) fn human(content: Vec<PlainBlock>) -> IncomingEntry {
    let human = HumanId::from_uuid(next_uuid()).unwrap();
    HumanMessage::new(entry_id(), human, content)
        .unwrap()
        .into()
}

pub(crate) fn agent_request(content: Vec<PlainBlock>) -> IncomingEntry {
    let request = RequestId::from_uuid(next_uuid()).unwrap();
    AgentRequest::new(
        entry_id(),
        agent_id(),
        Author::new("requester").unwrap(),
        ConversationId::from_uuid(next_uuid()).unwrap(),
        RequestOrigin::request(request),
        content,
    )
    .unwrap()
    .into()
}

pub(crate) fn agent_mail(content: Vec<PlainBlock>) -> IncomingEntry {
    AgentMail::new(
        entry_id(),
        agent_id(),
        Author::new("sender").unwrap(),
        ConversationId::from_uuid(next_uuid()).unwrap(),
        content,
    )
    .unwrap()
    .into()
}

pub(crate) fn as_turn_entry(entry: IncomingEntry) -> TurnEntry {
    match entry {
        IncomingEntry::HumanMessage(message) => message.into(),
        IncomingEntry::AgentRequest(request) => request.into(),
        IncomingEntry::AgentMail(mail) => mail.into(),
        other => panic!("fixture entry of a kind outside the three: {other:?}"),
    }
}

pub(crate) fn input_turn(entries: Vec<IncomingEntry>) -> Turn {
    Turn::InputTurn(InputTurn::new(turn_id(), entries).unwrap())
}

pub(crate) fn human_turn(content: Vec<PlainBlock>) -> Turn {
    input_turn(vec![human(content)])
}

pub(crate) fn content_text(value: &str) -> ContentBlock {
    ContentBlock::Text(TextBlock::new(block_id(), value).unwrap())
}

pub(crate) fn content_image(media_type: MediaType) -> ContentBlock {
    ContentBlock::Image(image(media_type))
}

pub(crate) fn finished(blocks: Vec<ContentBlock>) -> TurnEntry {
    TurnEntry::ModelResponse(response(blocks, ResponseEnding::finished()))
}

pub(crate) fn response(blocks: Vec<ContentBlock>, ending: ResponseEnding) -> ModelResponse {
    ModelResponse::new(entry_id(), blocks, ending, ResponseMetadata::new()).unwrap()
}

pub(crate) fn ai_turn(id: TurnId, agent: AgentId, entries: Vec<TurnEntry>) -> Turn {
    let author = Author::new("target").unwrap();
    Turn::AITurn(AITurn::from_parts(id, agent, author, entries).unwrap())
}

pub(crate) fn tool_call(name: &str) -> RuntimeToolCall {
    RuntimeToolCall::new(
        block_id(),
        ToolName::new(name).unwrap(),
        OpaqueJson::parse("{}").unwrap(),
    )
    .unwrap()
}

pub(crate) fn tool_results(calls: &[(&RuntimeToolCall, Vec<PlainBlock>)]) -> RuntimeToolResults {
    let results = calls
        .iter()
        .map(|(call, content)| {
            RuntimeToolResult::new(
                block_id(),
                call.id(),
                call.name().clone(),
                false,
                content.clone(),
            )
            .unwrap()
        })
        .collect();
    RuntimeToolResults::new(entry_id(), results).unwrap()
}

pub(crate) struct Fixture {
    pub(crate) agent: AgentId,
    pub(crate) conversation: ConversationId,
}

impl Fixture {
    pub(crate) fn new() -> Self {
        Self {
            agent: agent_id(),
            conversation: ConversationId::from_uuid(next_uuid()).unwrap(),
        }
    }

    pub(crate) fn project(&self, turns: Vec<Turn>) -> ProjectedThread {
        let conversation = Conversation::from_parts(self.conversation, turns).unwrap();
        Perspective::of(self.agent).project(&conversation)
    }

    pub(crate) fn project_with_partial_output(&self, content: ImageBlock) -> ProjectedThread {
        let other = agent_id();
        let turn = turn_id();
        let entry = entry_id();
        let turns = vec![ai_turn(turn, other, vec![])];
        let mut draft = ResponseDraft::new(turn, entry);
        for op in [
            DeltaOp::response_started(ResponseMetadata::new()),
            DeltaOp::block_inserted(content),
        ] {
            draft.apply(&ResponseDelta::new(turn, entry, op)).unwrap();
        }
        Perspective::of(self.agent)
            .project_pending(self.conversation, &turns, &[(&draft).into()])
            .unwrap()
    }
}

pub(crate) fn thread_of(turns: Vec<Turn>) -> ProjectedThread {
    Fixture::new().project(turns)
}

pub(crate) fn human_thread(content: Vec<PlainBlock>) -> ProjectedThread {
    thread_of(vec![human_turn(content)])
}

pub(crate) fn provider_result_blocks(media_type: MediaType) -> Vec<ContentBlock> {
    let call = ProviderToolCall::new(block_id());
    let result = ProviderToolResult::new(block_id(), call.id(), vec![plain_image(media_type)]);
    vec![
        ContentBlock::ProviderToolCall(call),
        ContentBlock::ProviderToolResult(result.unwrap()),
    ]
}

pub(crate) fn provider_record_block(media_type: MediaType) -> ContentBlock {
    let record = ProviderToolExecutionRecord::new(block_id())
        .with_content(vec![plain_image(media_type)])
        .unwrap();
    ContentBlock::ProviderToolExecutionRecord(record)
}
