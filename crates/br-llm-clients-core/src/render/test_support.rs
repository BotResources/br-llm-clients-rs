use br_llm_messages::{
    AITurn, AgentId, AgentMail, Author, BlockId, ContentBlock, ConversationId, DeltaOp, EntryId,
    ExternalId, IncomingEntry, ItemRef, OpaqueJson, PlainBlock, ProjectedThread, ReplayFormat,
    ReplayMaterial, ReplayPayload, RequestId, ResponseDelta, ResponseDraft, ResponseEnding,
    ResponseMetadata, RuntimeToolCall, RuntimeToolResult, RuntimeToolResults, StructuredBlock,
    TaskId, TextBlock, ToolName, Turn, TurnClosure, TurnEntry, TurnId,
};

use super::{RenderedHistory, RenderedPart, RenderedResultPart, RenderedRole, render_history};
use crate::thread_fixtures::{
    Fixture, agent_id, block_id, entry_id, human, input_turn, text_block, turn_id,
};
use crate::{ClientError, ClientRequest, OutputMode};

const PREFIX: &str = "01890a5d-ac96-774b-bcce-";

fn fixed(kind: char, n: u32) -> String {
    format!("{PREFIX}{kind}{n:011x}")
}

pub(super) fn fixed_block(n: u32) -> BlockId {
    BlockId::parse(&fixed('b', n)).unwrap()
}

pub(super) fn fixed_turn(n: u32) -> TurnId {
    TurnId::parse(&fixed('a', n)).unwrap()
}

pub(super) fn fixed_request(n: u32) -> RequestId {
    RequestId::parse(&fixed('d', n)).unwrap()
}

pub(super) fn fixed_task(n: u32) -> TaskId {
    TaskId::parse(&fixed('f', n)).unwrap()
}

pub(super) fn fixed_conversation(n: u32) -> ConversationId {
    ConversationId::parse(&fixed('9', n)).unwrap()
}

pub(super) fn fixed_entry(n: u32) -> EntryId {
    EntryId::parse(&fixed('e', n)).unwrap()
}

pub(super) fn call(n: u32, tool: &str) -> RuntimeToolCall {
    RuntimeToolCall::new(
        fixed_block(n),
        ToolName::new(tool).unwrap(),
        OpaqueJson::parse(r#"{"q":1}"#).unwrap(),
    )
    .unwrap()
}

pub(super) fn call_external(n: u32, tool: &str, external: &str) -> RuntimeToolCall {
    call(n, tool).with_external_id(ExternalId::new(external).unwrap())
}

pub(super) fn replay(text: &str) -> ReplayMaterial {
    ReplayMaterial::new(
        ReplayFormat::new("test/1").unwrap(),
        ReplayPayload::text(text),
    )
}

pub(super) fn text(value: &str) -> ContentBlock {
    ContentBlock::Text(TextBlock::new(block_id(), value).unwrap())
}

pub(super) fn text_with_replay(value: &str) -> ContentBlock {
    let block = TextBlock::new(block_id(), value)
        .unwrap()
        .with_replay(replay("sealed"));
    ContentBlock::Text(block)
}

pub(super) fn text_with_item_ref(value: &str) -> ContentBlock {
    let block = TextBlock::new(block_id(), value)
        .unwrap()
        .with_item_ref(ItemRef::new("item-1").unwrap());
    ContentBlock::Text(block)
}

pub(super) fn structured(json: &str) -> StructuredBlock {
    StructuredBlock::new(block_id(), OpaqueJson::parse(json).unwrap()).unwrap()
}

pub(super) fn calls_response(calls: &[RuntimeToolCall]) -> TurnEntry {
    let blocks = calls
        .iter()
        .cloned()
        .map(ContentBlock::RuntimeToolCall)
        .collect();
    TurnEntry::ModelResponse(crate::thread_fixtures::response(
        blocks,
        ResponseEnding::tool_use(),
    ))
}

pub(super) fn blocks_response(blocks: Vec<ContentBlock>) -> TurnEntry {
    TurnEntry::ModelResponse(crate::thread_fixtures::response(
        blocks,
        ResponseEnding::finished(),
    ))
}

pub(super) fn exhausted_response(blocks: Vec<ContentBlock>) -> TurnEntry {
    TurnEntry::ModelResponse(crate::thread_fixtures::response(
        blocks,
        ResponseEnding::output_exhaustion(),
    ))
}

pub(super) fn result(
    call: &RuntimeToolCall,
    is_error: bool,
    content: Vec<PlainBlock>,
) -> RuntimeToolResult {
    RuntimeToolResult::new(
        block_id(),
        call.id(),
        call.name().clone(),
        is_error,
        content,
    )
    .unwrap()
}

pub(super) fn results(list: Vec<RuntimeToolResult>) -> TurnEntry {
    TurnEntry::RuntimeToolResults(RuntimeToolResults::new(entry_id(), list).unwrap())
}

pub(super) fn closure(reason: br_llm_messages::ClosureReason) -> TurnEntry {
    TurnEntry::TurnClosure(TurnClosure::new(entry_id(), reason))
}

pub(super) fn human_entry(value: &str) -> IncomingEntry {
    human(vec![text_block(value)])
}

pub(super) fn human_message_content(content: Vec<PlainBlock>) -> IncomingEntry {
    human(content)
}

pub(super) fn human_turn_of(value: &str) -> Turn {
    input_turn(vec![human_entry(value)])
}

pub(super) fn target_turn(fixture: &Fixture, id: TurnId, entries: Vec<TurnEntry>) -> Turn {
    Turn::AITurn(
        AITurn::from_parts(id, fixture.agent, Author::new("target").unwrap(), entries).unwrap(),
    )
}

pub(super) fn other_agent_turn(label: &str, text_value: &str) -> Turn {
    other_agent_turn_of(agent_id(), label, vec![text_block(text_value)])
}

pub(super) fn other_agent_turn_of(agent: AgentId, label: &str, content: Vec<PlainBlock>) -> Turn {
    let entries = vec![blocks_response(
        content.into_iter().map(ContentBlock::from).collect(),
    )];
    Turn::AITurn(
        AITurn::from_parts(turn_id(), agent, Author::new(label).unwrap(), entries).unwrap(),
    )
}

pub(super) fn request_for(fixture: &Fixture, turns: Vec<Turn>, reply: TurnId) -> ClientRequest {
    ClientRequest::new(
        None,
        fixture.project(turns),
        reply,
        entry_id(),
        vec![],
        OutputMode::Plain,
    )
    .unwrap()
}

pub(super) fn request_of_thread(thread: ProjectedThread, reply: TurnId) -> ClientRequest {
    ClientRequest::new(None, thread, reply, entry_id(), vec![], OutputMode::Plain).unwrap()
}

pub(super) fn render(
    fixture: &Fixture,
    turns: Vec<Turn>,
    reply: TurnId,
) -> Result<RenderedHistory, ClientError> {
    render_history(&request_for(fixture, turns, reply))
}

pub(super) fn render_ok(fixture: &Fixture, turns: Vec<Turn>, reply: TurnId) -> RenderedHistory {
    render(fixture, turns, reply).unwrap()
}

pub(super) fn invalid_reason(outcome: Result<RenderedHistory, ClientError>) -> String {
    match outcome {
        Err(ClientError::InvalidMessages { reason }) => reason,
        other => panic!("expected InvalidMessages, got {other:?}"),
    }
}

fn describe_part(part: &RenderedPart) -> String {
    match part {
        RenderedPart::Text(text) => text.clone(),
        RenderedPart::Image(_) => "<image>".to_owned(),
        RenderedPart::ToolCall { call, wire_id } => {
            format!("call {wire_id} {}", call.name().as_str())
        }
        RenderedPart::ToolResult {
            wire_id,
            tool_name,
            is_error,
            parts,
        } => {
            let inner: Vec<String> = parts
                .iter()
                .map(|part| match part {
                    RenderedResultPart::Text(text) => text.clone(),
                    RenderedResultPart::Image(_) => "<image>".to_owned(),
                })
                .collect();
            format!(
                "result {wire_id} {} error={is_error} [{}]",
                tool_name.as_str(),
                inner.join(" | ")
            )
        }
        RenderedPart::Block(block) => format!("block {}", block.kind().as_str()),
    }
}

pub(super) fn view(history: &RenderedHistory) -> Vec<(char, Vec<String>)> {
    history
        .messages()
        .iter()
        .map(|message| {
            let role = match message.role() {
                RenderedRole::User => 'U',
                RenderedRole::Assistant => 'A',
            };
            (role, message.parts().iter().map(describe_part).collect())
        })
        .collect()
}

pub(super) fn strs(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

pub(super) fn agent_mail_entry(label: &str, content: Vec<PlainBlock>) -> IncomingEntry {
    IncomingEntry::AgentMail(
        AgentMail::new(
            entry_id(),
            agent_id(),
            Author::new(label).unwrap(),
            br_llm_messages::ConversationId::parse(&fixed('9', 77)).unwrap(),
            content,
        )
        .unwrap(),
    )
}

pub(super) fn partial_output_thread(
    fixture: &Fixture,
    label: &str,
    value: &str,
) -> ProjectedThread {
    let other = agent_id();
    let turn = turn_id();
    let entry = entry_id();
    let turns = vec![Turn::AITurn(
        AITurn::from_parts(turn, other, Author::new(label).unwrap(), vec![]).unwrap(),
    )];
    let mut draft = ResponseDraft::new(turn, entry);
    for op in [
        DeltaOp::response_started(ResponseMetadata::new()),
        DeltaOp::block_inserted(TextBlock::new(block_id(), value).unwrap()),
    ] {
        draft.apply(&ResponseDelta::new(turn, entry, op)).unwrap();
    }
    br_llm_messages::Perspective::of(fixture.agent)
        .project_pending(fixture.conversation, &turns, &[(&draft).into()])
        .unwrap()
}
