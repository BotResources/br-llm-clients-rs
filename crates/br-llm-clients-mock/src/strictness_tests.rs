use br_llm_clients_core::{
    CacheRequirement, ClientError, ClientRequest, ClientSettings, MaxOutputTokens, OutputMode,
    ReasoningLevel,
};
use br_llm_messages::MediaType;

use crate::strictness::check;
use crate::test_support::*;
use crate::{Accepted, Capabilities, CapabilitiesSpec};

const CAPABILITY_RULES: usize = 8;

fn breaking_settings() -> ClientSettings {
    settings_with(ReasoningLevel::Low, CacheRequirement::Required, 5)
}

fn breaking_request() -> ClientRequest {
    request_of(
        vec![
            crate::fixtures::text_block("look"),
            crate::fixtures::plain_image(MediaType::Png),
        ],
        vec![tool()],
        shape(),
    )
}

fn capabilities_breaking_from(first: usize) -> Capabilities {
    caps(|spec| {
        if first == 0 {
            spec.reasoning_levels = Accepted::Only(vec![]);
        }
        if first <= 1 {
            spec.caching = false;
        }
        if first <= 2 {
            spec.max_output_tokens = MaxOutputTokens::new(1).unwrap();
        }
        if first <= 3 {
            spec.tools = false;
        }
        if first <= 4 {
            spec.structured_output = false;
        }
        if first <= 5 {
            spec.structured_output_with_tools = false;
        }
        if first <= 6 {
            spec.images = false;
        }
        if first == 7 {
            spec.image_media_types = Accepted::Only(vec![MediaType::Gif]);
        }
    })
}

fn expected_refusal(rule: usize) -> ClientError {
    match rule {
        0 => ClientError::ReasoningNotSupported {
            level: ReasoningLevel::Low,
            reason: String::new(),
        },
        1 => ClientError::CacheNotSupported,
        2 => ClientError::MaxOutputTokensTooLarge { limit: Some(1) },
        3 => ClientError::ToolsNotSupported,
        4 => ClientError::StructuredOutputNotSupported,
        5 => ClientError::StructuredOutputWithToolsNotSupported,
        6 => ClientError::ImageNotSupported,
        _ => ClientError::ImageMediaTypeNotSupported {
            media_type: MediaType::Png,
        },
    }
}

fn same_case(left: &ClientError, right: &ClientError) -> bool {
    match (left, right) {
        (
            ClientError::ReasoningNotSupported { level: a, .. },
            ClientError::ReasoningNotSupported { level: b, .. },
        ) => a == b,
        _ => left == right,
    }
}

fn invalid_with_everything_else_broken() -> ClientRequest {
    let agent = crate::fixtures::agent_id();
    let history = crate::fixtures::project(
        agent,
        vec![
            crate::fixtures::unanswered_call_turn(agent),
            crate::fixtures::human_turn(vec![crate::fixtures::plain_image(MediaType::Png)]),
        ],
    );
    ClientRequest::new(
        None,
        history,
        crate::fixtures::turn_id(),
        crate::fixtures::entry_id(),
        vec![tool()],
        shape(),
    )
    .unwrap()
}

#[test]
fn given_a_request_breaking_capability_rules_when_checked_then_the_earliest_listed_rule_is_given() {
    for first in 0..CAPABILITY_RULES {
        let error = check(
            &capabilities_breaking_from(first),
            &breaking_settings(),
            &breaking_request(),
        )
        .unwrap_err();
        assert!(
            same_case(&error, &expected_refusal(first)),
            "rule {first} gave {error:?}"
        );
    }
}

#[test]
fn given_a_history_that_does_not_render_when_any_capability_rule_is_broken_then_invalid_messages_is_given()
 {
    let invalid = invalid_with_everything_else_broken();
    let expected = render_error(&invalid);
    assert!(matches!(expected, ClientError::InvalidMessages { .. }));
    for first in 0..CAPABILITY_RULES {
        assert_eq!(
            check(
                &capabilities_breaking_from(first),
                &breaking_settings(),
                &invalid
            ),
            Err(expected.clone()),
            "rule {first}"
        );
    }
}

fn render_error(request: &ClientRequest) -> ClientError {
    br_llm_clients_core::render_history(request).unwrap_err()
}

#[test]
fn given_a_history_that_renders_when_no_capability_rule_is_broken_then_the_check_passes() {
    assert_eq!(
        check(
            &everything_caps(),
            &breaking_settings(),
            &breaking_request()
        ),
        Ok(())
    );
}

#[test]
fn given_the_mock_when_a_history_does_not_render_then_the_call_gives_the_error_of_render_history() {
    let client = crate::MockClient::new(settings(), capabilities_breaking_from(0));
    let invalid = invalid_with_everything_else_broken();
    let (outcome, sink) = run(&client, &invalid);
    assert_eq!(error_of(&outcome), render_error(&invalid));
    assert!(sink.is_empty());
}

#[test]
fn given_an_empty_set_of_levels_when_a_valid_call_is_made_then_reasoning_not_supported() {
    let client = crate::MockClient::new(settings(), capabilities_breaking_from(0));
    let (outcome, _) = run(&client, &request());
    assert!(matches!(
        error_of(&outcome),
        ClientError::ReasoningNotSupported { .. }
    ));
}

#[test]
fn given_a_history_starting_with_an_assistant_message_when_called_then_the_mock_accepts_it() {
    let agent = crate::fixtures::agent_id();
    let history = crate::fixtures::project(
        agent,
        vec![
            crate::fixtures::answered_turn(agent, "first"),
            crate::fixtures::human_turn(vec![crate::fixtures::text_block("next")]),
        ],
    );
    let accepted = ClientRequest::new(
        None,
        history,
        crate::fixtures::turn_id(),
        crate::fixtures::entry_id(),
        vec![],
        OutputMode::Plain,
    )
    .unwrap();
    let client = mock();
    client.push_reply(crate::Reply::text("fine").build().unwrap());
    let (outcome, _) = run(&client, &accepted);
    assert_eq!(text_of(&outcome), "fine");
}

#[test]
fn given_a_tool_call_in_the_history_when_the_request_declares_no_tool_then_the_mock_accepts_it() {
    let agent = crate::fixtures::agent_id();
    let history = crate::fixtures::project(
        agent,
        vec![
            crate::fixtures::tool_exchange_turn(agent),
            crate::fixtures::human_turn(vec![crate::fixtures::text_block("go on")]),
        ],
    );
    let accepted = ClientRequest::new(
        None,
        history,
        crate::fixtures::turn_id(),
        crate::fixtures::entry_id(),
        vec![],
        OutputMode::Plain,
    )
    .unwrap();
    let client = mock();
    client.push_reply(crate::Reply::text("fine").build().unwrap());
    assert_eq!(text_of(&run(&client, &accepted).0), "fine");
}

#[test]
fn given_a_call_for_a_name_not_declared_in_the_tools_when_served_then_the_mock_does_not_check_it() {
    let client = mock();
    client.push_reply(
        crate::Reply::tool_call("not_declared", "{}")
            .build()
            .unwrap(),
    );
    let with_tool = request_of(
        vec![crate::fixtures::text_block("hi")],
        vec![tool()],
        OutputMode::Plain,
    );
    assert!(run(&client, &with_tool).0.result().is_ok());
}

#[test]
fn given_settings_that_break_a_rule_when_a_handle_is_used_then_every_call_of_the_handle_gives_the_same_refusal()
 {
    let client = crate::MockClient::new(breaking_settings(), capabilities_breaking_from(1));
    let first = error_of(&run(&client, &request()).0);
    let second = error_of(&run(&client, &long_request()).0);
    assert_eq!(first, ClientError::CacheNotSupported);
    assert_eq!(second, ClientError::CacheNotSupported);
}

#[test]
fn given_a_spec_with_a_changed_field_when_checked_then_only_that_field_refuses() {
    let spec = CapabilitiesSpec {
        tools: false,
        ..CapabilitiesSpec::everything()
    };
    let capabilities = Capabilities::new(spec).unwrap();
    let with_tool = request_of(
        vec![crate::fixtures::text_block("hi")],
        vec![tool()],
        OutputMode::Plain,
    );
    assert_eq!(
        check(&capabilities, &settings(), &with_tool),
        Err(ClientError::ToolsNotSupported)
    );
    assert_eq!(check(&capabilities, &settings(), &request()), Ok(()));
}
