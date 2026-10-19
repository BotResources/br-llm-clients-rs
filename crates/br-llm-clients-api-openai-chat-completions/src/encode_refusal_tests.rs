use br_llm_clients_common_call::Dialect;
use br_llm_clients_core::{ClientError, OutputMode};
use br_llm_messages::{AssistantBlock, ImageMime, Thinking, WireMessage};
use serde_json::json;

use crate::test_support::{
    assistant, assistant_call, assistant_text, dialect, dialect_for, encode_error, image,
    request_of, request_with, shape, text, tool, tool_result, user, user_text,
};
use crate::{ChatCompletionsOptions, ReasoningLevel, ReasoningTable};

fn refusing(level: ReasoningLevel) -> crate::ChatCompletionsDialect {
    dialect_for(
        level,
        ChatCompletionsOptions::new(ReasoningTable::openai_effort_without_none()),
    )
}

fn assistant_last() -> Vec<WireMessage> {
    vec![user_text("go"), assistant(vec![assistant_text("so")])]
}

#[test]
fn given_a_level_the_table_refuses_when_encoded_then_every_call_gives_reasoning_not_supported() {
    let dialect = refusing(ReasoningLevel::Off);
    let request = request_of(vec![user_text("hello")]);
    for _ in 0..3 {
        match encode_error(&dialect, &request) {
            ClientError::ReasoningNotSupported { level, reason } => {
                assert_eq!(level, ReasoningLevel::Off);
                assert!(reason.contains("table"));
            }
            other => panic!("unexpected {other:?}"),
        }
    }
}

#[test]
fn given_an_assistant_message_last_when_encoded_then_the_messages_are_invalid() {
    let error = encode_error(&dialect(), &request_of(assistant_last()));
    assert!(matches!(error, ClientError::InvalidMessages { .. }));
}

#[test]
fn given_a_last_assistant_message_holding_only_thinking_when_encoded_then_it_is_still_refused() {
    let thinking = AssistantBlock::Thinking(Thinking {
        text: "hmm".to_owned(),
        signature: None,
    });
    let request = request_of(vec![user_text("go"), assistant(vec![thinking])]);
    assert!(matches!(
        encode_error(&dialect(), &request),
        ClientError::InvalidMessages { .. }
    ));
}

#[test]
fn given_a_relay_last_when_encoded_then_it_is_accepted() {
    let request = request_of(vec![
        user_text("go"),
        WireMessage::Relay {
            content: vec![text("note")],
        },
    ]);
    assert!(dialect().encode(&request).is_ok());
}

#[test]
fn given_a_user_message_with_results_last_when_encoded_then_it_is_accepted() {
    let request = request_of(vec![
        user_text("go"),
        assistant(vec![assistant_call("c1", "lookup", json!({}))]),
        user(vec![tool_result("c1", "lookup", &["x"], false)]),
    ]);
    assert!(dialect().encode(&request).is_ok());
}

#[test]
fn given_shape_names_outside_the_rule_when_encoded_then_the_output_shape_is_rejected() {
    let too_long = "a".repeat(65);
    for name in [
        too_long.as_str(),
        "has space",
        "dot.name",
        "caf\u{e9}",
        "slash/name",
        "semi;colon",
    ] {
        let request = request_with(
            None,
            vec![user_text("hello")],
            Vec::new(),
            shape(name, None, Vec::new()),
        );
        assert!(
            matches!(
                encode_error(&dialect(), &request),
                ClientError::OutputShapeRejected { .. }
            ),
            "{name}"
        );
    }
}

#[test]
fn given_shape_names_inside_the_rule_when_encoded_then_they_are_accepted() {
    let longest = "a".repeat(64);
    for name in ["a", longest.as_str(), "Answer_v2-final", "0_9-Zz"] {
        let request = request_with(
            None,
            vec![user_text("hello")],
            Vec::new(),
            shape(name, None, Vec::new()),
        );
        assert!(dialect().encode(&request).is_ok(), "{name}");
    }
}

#[test]
fn given_several_refusals_when_encoded_then_the_first_in_the_documented_order_is_returned() {
    let bad_shape = shape("bad name", None, Vec::new());
    let request = request_with(None, assistant_last(), Vec::new(), bad_shape.clone());
    assert!(matches!(
        encode_error(&refusing(ReasoningLevel::Off), &request),
        ClientError::ReasoningNotSupported { .. }
    ));
    assert!(matches!(
        encode_error(&dialect(), &request),
        ClientError::InvalidMessages { .. }
    ));
    let only_shape = request_with(None, vec![user_text("hello")], Vec::new(), bad_shape);
    assert!(matches!(
        encode_error(&dialect(), &only_shape),
        ClientError::OutputShapeRejected { .. }
    ));
}

#[test]
fn given_unchecked_matters_when_encoded_then_they_are_left_to_the_host() {
    let many_tools = (0..200)
        .map(|index| tool(&format!("tool_{index}"), "", Vec::new()))
        .collect();
    let request = request_with(
        None,
        vec![user(vec![
            image(ImageMime::Webp, "AAAA"),
            image(ImageMime::Gif, "AAAA"),
            image(ImageMime::Png, "AAAA"),
        ])],
        many_tools,
        OutputMode::Plain,
    );
    let custom = ChatCompletionsOptions::new(ReasoningTable::openai_effort());
    assert!(
        dialect_for(ReasoningLevel::Max, custom)
            .encode(&request)
            .is_ok()
    );
}
