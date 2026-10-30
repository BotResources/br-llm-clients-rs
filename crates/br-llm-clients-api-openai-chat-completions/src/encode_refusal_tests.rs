use br_llm_clients_core::{ClientError, OutputMode};
use br_llm_messages::{MediaType, Turn};
use serde_json::json;

use crate::test_support::{
    dialect, dialect_for, encode, encode_error, request_of, request_with, shape, tool,
};
use crate::test_thread::{
    answer, call, call_block, calling, earlier, human, image, in_progress, plain_image,
    provider_call, result_of, results, said, text, texts, user,
};
use crate::{ChatCompletionsOptions, ReasoningLevel, ReasoningTable};

fn refusing(level: ReasoningLevel) -> crate::ChatCompletionsDialect {
    dialect_for(
        level,
        ChatCompletionsOptions::new(ReasoningTable::openai_effort_without_none()),
    )
}

fn assistant_last() -> Vec<Turn> {
    vec![user("go"), said("so")]
}

fn reason_of(error: ClientError) -> String {
    match error {
        ClientError::InvalidMessages { reason } => reason,
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn given_a_level_the_table_refuses_when_encoded_then_every_call_gives_reasoning_not_supported() {
    let dialect = refusing(ReasoningLevel::Off);
    let request = request_of(vec![user("hello")]);
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
    let reason = reason_of(encode_error(&dialect(), &request_of(assistant_last())));
    assert!(reason.contains("prefill"));
}

#[test]
fn given_a_user_message_with_results_last_when_encoded_then_it_is_accepted() {
    let lookup = call("c1", "lookup", json!({}));
    let request = request_of(vec![
        user("go"),
        in_progress(vec![
            calling(vec![call_block(&lookup)]),
            results(vec![result_of(&lookup, texts(&["x"]), false)]),
        ]),
    ]);
    assert!(encode(&dialect(), &request).is_ok());
}

#[test]
fn given_an_image_in_an_assistant_message_when_encoded_then_the_reason_names_it() {
    let request = request_of(vec![
        user("go"),
        earlier(vec![answer(vec![
            text("see"),
            image(MediaType::Png, "QUJD"),
        ])]),
        user("again"),
    ]);
    let reason = reason_of(encode_error(&dialect(), &request));
    assert!(reason.contains("image"));
    assert!(reason.contains("assistant"));
}

#[test]
fn given_an_image_in_the_result_of_a_tool_when_encoded_then_the_reason_names_it() {
    let lookup = call("c1", "lookup", json!({}));
    let request = request_of(vec![
        user("go"),
        earlier(vec![
            calling(vec![call_block(&lookup)]),
            results(vec![result_of(
                &lookup,
                vec![plain_image(MediaType::Png, "QUJD")],
                false,
            )]),
        ]),
    ]);
    let reason = reason_of(encode_error(&dialect(), &request));
    assert!(reason.contains("image"));
    assert!(reason.contains("result"));
}

#[test]
fn given_a_provider_tool_block_of_the_turn_in_progress_when_encoded_then_the_reason_names_its_kind()
{
    let lookup = call("c1", "lookup", json!({}));
    let request = request_of(vec![
        user("go"),
        in_progress(vec![
            calling(vec![provider_call(), call_block(&lookup)]),
            results(vec![result_of(&lookup, texts(&["x"]), false)]),
        ]),
    ]);
    let reason = reason_of(encode_error(&dialect(), &request));
    assert!(reason.contains("provider_tool_call"), "{reason}");
}

#[test]
fn given_a_provider_tool_block_of_an_earlier_turn_when_encoded_then_it_is_not_in_the_rendering() {
    let request = request_of(vec![
        user("go"),
        earlier(vec![answer(vec![provider_call(), text("done")])]),
        user("again"),
    ]);
    assert!(encode(&dialect(), &request).is_ok());
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
            vec![user("hello")],
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
            vec![user("hello")],
            Vec::new(),
            shape(name, None, Vec::new()),
        );
        assert!(encode(&dialect(), &request).is_ok(), "{name}");
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
    let only_shape = request_with(None, vec![user("hello")], Vec::new(), bad_shape);
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
        vec![human(vec![
            plain_image(MediaType::Webp, "AAAA"),
            plain_image(MediaType::Gif, "AAAA"),
            plain_image(MediaType::Png, "AAAA"),
        ])],
        many_tools,
        OutputMode::Plain,
    );
    let custom = ChatCompletionsOptions::new(ReasoningTable::openai_effort());
    assert!(encode(&dialect_for(ReasoningLevel::Max, custom), &request).is_ok());
}
