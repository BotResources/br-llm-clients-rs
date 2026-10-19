use br_llm_clients_core::{OutputMode, ParameterKind};
use serde_json::{Value, json};

use crate::test_support::{dialect, encoded_body, parameter, request_with, shape, tool, user_text};

fn format_of(output: OutputMode) -> Value {
    let request = request_with(None, vec![user_text("hello")], Vec::new(), output);
    encoded_body(&dialect(), &request)["response_format"].clone()
}

#[test]
fn given_a_structured_shape_when_encoded_then_response_format_is_a_json_schema_without_strict() {
    let output = shape(
        "answer",
        Some("the final answer"),
        vec![
            parameter("value", "the value", true, ParameterKind::Integer),
            parameter("note", "", false, ParameterKind::String),
        ],
    );
    assert_eq!(
        format_of(output),
        json!({"type": "json_schema", "json_schema": {
            "name": "answer",
            "description": "the final answer",
            "schema": {
                "type": "object",
                "properties": {
                    "value": {"type": "integer", "description": "the value"},
                    "note": {"type": "string"}
                },
                "required": ["value"]
            }
        }})
    );
}

#[test]
fn given_no_shape_description_when_encoded_then_the_description_member_is_not_written() {
    let none = format_of(shape("answer", None, Vec::new()));
    assert!(none["json_schema"].get("description").is_none());
    let empty = format_of(shape("answer", Some(""), Vec::new()));
    assert!(empty["json_schema"].get("description").is_none());
}

#[test]
fn given_a_shape_with_no_parameter_when_encoded_then_the_schema_is_an_empty_object_schema() {
    let format = format_of(shape("empty", None, Vec::new()));
    assert_eq!(
        format["json_schema"]["schema"],
        json!({"type": "object", "properties": {}})
    );
}

#[test]
fn given_plain_output_when_encoded_then_no_response_format_is_written() {
    let request = request_with(
        None,
        vec![user_text("hello")],
        Vec::new(),
        OutputMode::Plain,
    );
    assert!(
        encoded_body(&dialect(), &request)
            .get("response_format")
            .is_none()
    );
}

#[test]
fn given_a_shape_and_tools_when_encoded_then_both_members_are_written() {
    let request = request_with(
        None,
        vec![user_text("hello")],
        vec![tool("lookup", "", Vec::new())],
        shape("answer", None, Vec::new()),
    );
    let body = encoded_body(&dialect(), &request);
    assert!(body.get("tools").is_some());
    assert!(body.get("response_format").is_some());
}

#[test]
fn given_a_shape_when_encoded_then_the_strict_flag_is_nowhere_in_the_body() {
    let request = request_with(
        None,
        vec![user_text("hello")],
        Vec::new(),
        shape(
            "answer",
            None,
            vec![parameter("v", "", true, ParameterKind::String)],
        ),
    );
    assert!(
        !encoded_body(&dialect(), &request)
            .to_string()
            .contains("strict")
    );
}
