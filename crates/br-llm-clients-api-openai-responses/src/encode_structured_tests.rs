use br_llm_clients_core::{ClientError, LimitKind, ParameterKind};
use serde_json::json;

use crate::test_requests::{dialect, encode_body, parameter, shape, structured_request, tool};
use br_llm_clients_core::ReasoningLevel;

fn encode(
    shape: br_llm_clients_core::StructuredShape,
    tools: Vec<br_llm_clients_core::ToolSpec>,
) -> Result<serde_json::Value, ClientError> {
    encode_body(
        &dialect(ReasoningLevel::Off),
        &structured_request(shape, tools),
    )
}

fn answer_parameter() -> br_llm_clients_core::Parameter {
    parameter("answer", "the answer", true, ParameterKind::String)
}

#[test]
fn given_a_structured_output_when_encoded_then_text_format_is_a_json_schema() {
    let body = encode(
        shape(
            "final_answer",
            Some("the final answer"),
            vec![answer_parameter()],
        ),
        Vec::new(),
    )
    .expect("encoded");
    assert_eq!(
        body["text"],
        json!({
            "format": {
                "type": "json_schema",
                "name": "final_answer",
                "description": "the final answer",
                "schema": {
                    "type": "object",
                    "properties": { "answer": { "type": "string", "description": "the answer" } },
                    "required": ["answer"],
                    "additionalProperties": false
                },
                "strict": true
            }
        })
    );
}

#[test]
fn given_a_shape_without_description_when_encoded_then_the_description_is_not_sent() {
    let body = encode(shape("s", None, vec![answer_parameter()]), Vec::new()).expect("encoded");
    assert!(body["text"]["format"].get("description").is_none());
}

#[test]
fn given_an_empty_but_present_description_when_encoded_then_it_is_sent() {
    let body = encode(shape("s", Some(""), vec![answer_parameter()]), Vec::new()).expect("encoded");
    assert_eq!(body["text"]["format"]["description"], json!(""));
}

#[test]
fn given_a_shape_with_an_optional_parameter_when_encoded_then_strict_is_false() {
    let optional = parameter("note", "", false, ParameterKind::String);
    let body = encode(
        shape("s", None, vec![answer_parameter(), optional]),
        Vec::new(),
    )
    .expect("encoded");
    assert_eq!(body["text"]["format"]["strict"], json!(false));
}

#[test]
fn given_a_shape_with_a_nested_optional_parameter_when_encoded_then_strict_is_false() {
    let inner = crate::test_requests::parameters(vec![parameter(
        "inner",
        "",
        false,
        ParameterKind::String,
    )]);
    let outer = parameter(
        "outer",
        "",
        true,
        ParameterKind::Object { parameters: inner },
    );
    let body = encode(shape("s", None, vec![outer]), Vec::new()).expect("encoded");
    assert_eq!(body["text"]["format"]["strict"], json!(false));
}

#[test]
fn given_a_shape_without_parameters_when_encoded_then_the_schema_is_empty_and_strict() {
    let body = encode(shape("empty", None, Vec::new()), Vec::new()).expect("encoded");
    assert_eq!(body["text"]["format"]["strict"], json!(true));
    assert_eq!(body["text"]["format"]["schema"]["properties"], json!({}));
}

#[test]
fn given_a_name_with_a_character_outside_the_set_when_encoded_then_the_shape_is_rejected() {
    for name in [
        "has space",
        "dot.dot",
        "slash/slash",
        "accent\u{e9}",
        "plus+",
    ] {
        let refusal = encode(shape(name, None, Vec::new()), Vec::new()).expect_err("refused");
        assert!(
            matches!(refusal, ClientError::OutputShapeRejected { .. }),
            "{name}"
        );
    }
}

#[test]
fn given_a_name_of_the_allowed_set_when_encoded_then_it_is_sent_unchanged() {
    for name in ["abc_DEF-123", "_", "-", "9", "a"] {
        let body = encode(shape(name, None, Vec::new()), Vec::new()).expect("encoded");
        assert_eq!(body["text"]["format"]["name"], json!(name));
    }
}

#[test]
fn given_a_name_of_64_characters_when_encoded_then_it_is_accepted() {
    let name = "a".repeat(64);
    assert!(encode(shape(&name, None, Vec::new()), Vec::new()).is_ok());
}

#[test]
fn given_a_name_of_65_characters_when_encoded_then_the_limit_is_exceeded_of_kind_other() {
    let name = "a".repeat(65);
    let refusal = encode(shape(&name, None, Vec::new()), Vec::new()).expect_err("refused");
    assert!(matches!(
        refusal,
        ClientError::LimitExceeded {
            kind: LimitKind::Other,
            ..
        }
    ));
}

#[test]
fn given_a_long_name_with_a_bad_character_when_encoded_then_the_character_check_comes_first() {
    let name = format!("{} ", "a".repeat(70));
    let refusal = encode(shape(&name, None, Vec::new()), Vec::new()).expect_err("refused");
    assert!(matches!(refusal, ClientError::OutputShapeRejected { .. }));
}

#[test]
fn given_a_structured_output_with_tools_when_encoded_then_it_is_accepted_with_both_fields() {
    let body = encode(
        shape("s", None, vec![answer_parameter()]),
        vec![tool("search", "", Vec::new())],
    )
    .expect("accepted");
    assert!(body.get("text").is_some());
    assert_eq!(body["tools"].as_array().expect("tools").len(), 1);
}
