use br_llm_messages::{AssistantBlock, ToolName, WireMessage, WireUserBlock};

use crate::build_error::BuildError;
use crate::output_mode::OutputMode;
use crate::test_support::{text, tool};
use crate::{ClientRequest, ToolSpec};

fn user(value: &str) -> WireMessage {
    WireMessage::User {
        content: vec![WireUserBlock::Text { text: text(value) }],
    }
}

fn assistant(value: &str) -> WireMessage {
    WireMessage::Assistant {
        content: vec![AssistantBlock::Text { text: text(value) }],
    }
}

fn relay(value: &str) -> WireMessage {
    WireMessage::Relay {
        content: vec![text(value)],
    }
}

fn build(messages: Vec<WireMessage>, tools: Vec<ToolSpec>) -> Result<ClientRequest, BuildError> {
    ClientRequest::new(None, messages, tools, OutputMode::Plain)
}

#[test]
fn given_no_message_when_built_then_refused() {
    assert_eq!(build(vec![], vec![]), Err(BuildError::NoMessage));
}

#[test]
fn given_a_lone_relay_when_built_then_refused() {
    assert_eq!(
        build(vec![relay("r")], vec![]),
        Err(BuildError::MisplacedRelay)
    );
}

#[test]
fn given_a_relay_before_the_end_when_built_then_refused() {
    assert_eq!(
        build(vec![user("a"), relay("r"), assistant("b")], vec![]),
        Err(BuildError::MisplacedRelay)
    );
}

#[test]
fn given_two_relays_at_the_end_when_built_then_refused() {
    assert_eq!(
        build(vec![user("a"), relay("r"), relay("s")], vec![]),
        Err(BuildError::MisplacedRelay)
    );
}

#[test]
fn given_a_relay_last_after_another_message_when_built_then_accepted() {
    assert!(build(vec![user("a"), relay("r")], vec![]).is_ok());
}

#[test]
fn given_two_tools_with_one_name_when_built_then_refused_naming_the_tool() {
    assert_eq!(
        build(
            vec![user("a")],
            vec![tool("search"), tool("calc"), tool("search")]
        ),
        Err(BuildError::DuplicateToolName {
            name: ToolName::new("search").unwrap()
        })
    );
}

#[test]
fn given_no_system_prompt_and_no_tool_when_built_then_accepted() {
    let request = build(vec![user("a")], vec![]).unwrap();
    assert!(request.system().is_none());
    assert!(request.tools().is_empty());
}

#[test]
fn given_a_request_that_only_a_provider_would_refuse_when_built_then_accepted() {
    assert!(build(vec![assistant("a")], vec![]).is_ok());
    assert!(build(vec![user("a"), assistant("b")], vec![]).is_ok());
}

#[test]
fn given_a_request_when_read_then_the_fields_come_back_as_given() {
    let request = ClientRequest::new(
        Some(text("be brief")),
        vec![user("a")],
        vec![tool("search")],
        OutputMode::Plain,
    )
    .unwrap();
    assert_eq!(request.system(), Some(&text("be brief")));
    assert_eq!(request.messages(), &[user("a")]);
    assert_eq!(request.tools(), &[tool("search")]);
    assert_eq!(request.output(), &OutputMode::Plain);
}
