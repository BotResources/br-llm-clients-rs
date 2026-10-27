use br_llm_messages::{ContentBlock, ProjectedThread, ResponseEnding, ToolName};

use crate::build_error::BuildError;
use crate::output_mode::OutputMode;
use crate::test_support::{plain_text, tool};
use crate::thread_fixtures::{
    Fixture, ai_turn, entry_id, human_thread, human_turn, response, text_block, tool_call, turn_id,
};
use crate::{ClientRequest, ToolSpec};

fn thread() -> ProjectedThread {
    human_thread(vec![text_block("hello")])
}

fn build(history: ProjectedThread, tools: Vec<ToolSpec>) -> Result<ClientRequest, BuildError> {
    ClientRequest::new(
        None,
        history,
        turn_id(),
        entry_id(),
        tools,
        OutputMode::Plain,
    )
}

fn empty_thread() -> ProjectedThread {
    let fixture = Fixture::new();
    ProjectedThread::from_json(&format!(
        r#"{{"schema":"br-llm-messages-projected-thread/1","conversation":"{}","agent":"{}","turns":[]}}"#,
        fixture.conversation, fixture.agent
    ))
    .unwrap()
}

#[test]
fn given_a_thread_without_any_turn_when_built_then_refused() {
    let history = empty_thread();
    assert!(history.turns().is_empty());
    assert_eq!(build(history, vec![]), Err(BuildError::EmptyThread));
}

#[test]
fn given_a_thread_with_one_turn_when_built_then_accepted() {
    assert!(build(thread(), vec![]).is_ok());
}

#[test]
fn given_two_tools_with_one_name_when_built_then_refused_naming_the_tool() {
    assert_eq!(
        build(thread(), vec![tool("search"), tool("calc"), tool("search")]),
        Err(BuildError::DuplicateToolName {
            name: ToolName::new("search").unwrap()
        })
    );
}

#[test]
fn given_no_system_prompt_and_no_tool_when_built_then_accepted() {
    let request = build(thread(), vec![]).unwrap();
    assert!(request.system().is_none());
    assert!(request.tools().is_empty());
}

#[test]
fn given_a_thread_that_only_a_provider_or_the_rendering_would_refuse_when_built_then_accepted() {
    let fixture = Fixture::new();
    let reply = response(
        vec![ContentBlock::RuntimeToolCall(tool_call("search"))],
        ResponseEnding::tool_use(),
    );
    let open_with_unanswered_call = fixture.project(vec![
        human_turn(vec![text_block("a")]),
        ai_turn(turn_id(), fixture.agent, vec![reply.into()]),
    ]);
    assert!(build(open_with_unanswered_call, vec![]).is_ok());
}

#[test]
fn given_reply_identities_the_thread_does_not_know_when_built_then_accepted() {
    let history = thread();
    let unknown_turn = turn_id();
    let unknown_entry = entry_id();
    let request = ClientRequest::new(
        None,
        history,
        unknown_turn,
        unknown_entry,
        vec![],
        OutputMode::Plain,
    )
    .unwrap();
    assert_eq!(request.reply_turn(), unknown_turn);
    assert_eq!(request.reply_entry(), unknown_entry);
}

#[test]
fn given_a_request_when_read_then_the_fields_come_back_as_given() {
    let history = thread();
    let (reply_turn, reply_entry) = (turn_id(), entry_id());
    let request = ClientRequest::new(
        Some(plain_text("be brief")),
        history.clone(),
        reply_turn,
        reply_entry,
        vec![tool("search")],
        OutputMode::Plain,
    )
    .unwrap();
    assert_eq!(request.system(), Some(&plain_text("be brief")));
    assert_eq!(request.history(), &history);
    assert_eq!(request.reply_turn(), reply_turn);
    assert_eq!(request.reply_entry(), reply_entry);
    assert_eq!(request.tools(), &[tool("search")]);
    assert_eq!(request.output(), &OutputMode::Plain);
}
