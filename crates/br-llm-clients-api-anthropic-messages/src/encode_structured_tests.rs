use br_llm_clients_core::ParameterKind;
use serde_json::{Value, json};

use crate::test_fixtures::*;

fn config_of(output: br_llm_clients_core::OutputMode) -> Value {
    let request = request_with(None, vec![user("hello")], vec![], output);
    body_of(&dialect(), &request)
        .get("output_config")
        .cloned()
        .unwrap_or(Value::Null)
}

#[test]
fn given_a_plain_output_when_encoded_then_the_body_gets_no_format() {
    assert!(simple_body().get("output_config").is_none());
}

#[test]
fn given_a_shape_when_encoded_then_the_format_is_a_closed_json_schema_with_its_description() {
    let config = config_of(structured(
        Some("the final answer"),
        vec![
            param("value", "the value", true, ParameterKind::String),
            param("note", "", false, ParameterKind::Boolean),
        ],
    ));
    assert_eq!(
        config,
        json!({"format": {"type": "json_schema", "schema": {
            "type": "object",
            "description": "the final answer",
            "properties": {
                "value": {"type": "string", "description": "the value"},
                "note": {"type": "boolean"},
            },
            "required": ["value"],
            "additionalProperties": false,
        }}})
    );
}

#[test]
fn given_a_shape_when_encoded_then_the_name_of_the_shape_is_not_sent() {
    let request = request_with(
        None,
        vec![user("hello")],
        vec![],
        structured(None, vec![param("v", "", true, ParameterKind::String)]),
    );
    assert!(!body_of(&dialect(), &request).to_string().contains("answer"));
}

#[test]
fn given_a_shape_without_description_when_encoded_then_the_schema_has_none() {
    let config = config_of(structured(None, vec![]));
    assert!(config["format"]["schema"].get("description").is_none());
    assert_eq!(config["format"]["schema"]["properties"], json!({}));
}

#[test]
fn given_a_shape_with_an_empty_description_when_encoded_then_the_schema_has_none() {
    let config = config_of(structured(Some(""), vec![]));
    assert!(config["format"]["schema"].get("description").is_none());
}

#[test]
fn given_nested_objects_when_encoded_then_every_object_is_closed() {
    let leaf = parameters(vec![param("z", "", true, ParameterKind::Integer)]);
    let middle = parameters(vec![
        param(
            "leaf",
            "",
            true,
            ParameterKind::Object {
                parameters: leaf.clone(),
            },
        ),
        param(
            "leaves",
            "",
            true,
            ParameterKind::Array {
                items: Box::new(ParameterKind::Object { parameters: leaf }),
            },
        ),
    ]);
    let config = config_of(structured(
        None,
        vec![param(
            "middle",
            "",
            true,
            ParameterKind::Object { parameters: middle },
        )],
    ));
    let schema = &config["format"]["schema"];
    let middle = &schema["properties"]["middle"];
    assert_eq!(schema["additionalProperties"], json!(false));
    assert_eq!(middle["additionalProperties"], json!(false));
    assert_eq!(
        middle["properties"]["leaf"]["additionalProperties"],
        json!(false)
    );
    assert_eq!(
        middle["properties"]["leaves"]["items"]["additionalProperties"],
        json!(false)
    );
    assert_eq!(
        schema.to_string().matches("additionalProperties").count(),
        4
    );
}

#[test]
fn given_a_shape_and_a_reasoning_level_when_encoded_then_effort_and_format_share_the_config() {
    let request = request_with(
        None,
        vec![user("hello")],
        vec![],
        structured(None, vec![param("v", "", true, ParameterKind::String)]),
    );
    let dialect = dialect_with(
        br_llm_clients_core::ReasoningLevel::Low,
        br_llm_clients_core::CacheRequirement::NotRequired,
    );
    let config = body_of(&dialect, &request)["output_config"].clone();
    assert_eq!(config["effort"], json!("low"));
    assert_eq!(config["format"]["type"], json!("json_schema"));
    assert_eq!(config.as_object().expect("object").len(), 2);
}
