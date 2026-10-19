use br_llm_clients_core::{OutputMode, ParameterKind};
use serde_json::{Value, json};

use crate::test_fixtures::*;

fn tools_of(tools: Vec<br_llm_clients_core::ToolSpec>) -> Value {
    let request = request_with(None, vec![user("hello")], tools, OutputMode::Plain);
    body_of(&dialect(), &request)["tools"].clone()
}

#[test]
fn given_a_tool_when_encoded_then_it_has_name_description_and_an_object_input_schema() {
    let tools = tools_of(vec![tool(
        "weather",
        "gives the weather",
        vec![param("city", "the city", true, ParameterKind::String)],
    )]);
    assert_eq!(
        tools,
        json!([{
            "name": "weather",
            "description": "gives the weather",
            "input_schema": {
                "type": "object",
                "properties": {"city": {"type": "string", "description": "the city"}},
                "required": ["city"],
            },
        }])
    );
}

#[test]
fn given_an_empty_description_when_encoded_then_the_tool_has_no_description_field() {
    let tools = tools_of(vec![tool("ping", "", vec![])]);
    assert_eq!(
        tools,
        json!([{"name": "ping", "input_schema": {"type": "object", "properties": {}}}])
    );
}

#[test]
fn given_no_required_parameter_when_encoded_then_required_is_absent() {
    let tools = tools_of(vec![tool(
        "t",
        "",
        vec![param("a", "", false, ParameterKind::Integer)],
    )]);
    assert!(tools[0]["input_schema"].get("required").is_none());
}

#[test]
fn given_mixed_required_parameters_when_encoded_then_required_lists_them_in_order() {
    let tools = tools_of(vec![tool(
        "t",
        "",
        vec![
            param("c", "", true, ParameterKind::String),
            param("a", "", false, ParameterKind::String),
            param("b", "", true, ParameterKind::String),
        ],
    )]);
    assert_eq!(tools[0]["input_schema"]["required"], json!(["c", "b"]));
}

#[test]
fn given_every_scalar_kind_when_encoded_then_the_json_type_has_the_name_of_the_kind() {
    let tools = tools_of(vec![tool(
        "t",
        "",
        vec![
            param("s", "", true, ParameterKind::String),
            param("n", "", true, ParameterKind::Number),
            param("i", "", true, ParameterKind::Integer),
            param("b", "", true, ParameterKind::Boolean),
        ],
    )]);
    let properties = &tools[0]["input_schema"]["properties"];
    assert_eq!(properties["s"], json!({"type": "string"}));
    assert_eq!(properties["n"], json!({"type": "number"}));
    assert_eq!(properties["i"], json!({"type": "integer"}));
    assert_eq!(properties["b"], json!({"type": "boolean"}));
}

#[test]
fn given_an_enum_when_encoded_then_it_is_a_string_with_its_values_in_order() {
    let tools = tools_of(vec![tool(
        "t",
        "",
        vec![param(
            "unit",
            "",
            true,
            ParameterKind::Enum {
                values: vec!["kelvin".to_owned(), "celsius".to_owned()],
            },
        )],
    )]);
    assert_eq!(
        tools[0]["input_schema"]["properties"]["unit"],
        json!({"type": "string", "enum": ["kelvin", "celsius"]})
    );
}

#[test]
fn given_an_array_of_objects_when_encoded_then_items_hold_the_object_schema() {
    let inner = parameters(vec![
        param("x", "", true, ParameterKind::Number),
        param("y", "", false, ParameterKind::Number),
    ]);
    let tools = tools_of(vec![tool(
        "t",
        "",
        vec![param(
            "points",
            "a list",
            true,
            ParameterKind::Array {
                items: Box::new(ParameterKind::Object { parameters: inner }),
            },
        )],
    )]);
    assert_eq!(
        tools[0]["input_schema"]["properties"]["points"],
        json!({
            "type": "array",
            "description": "a list",
            "items": {
                "type": "object",
                "properties": {"x": {"type": "number"}, "y": {"type": "number"}},
                "required": ["x"],
            },
        })
    );
}

#[test]
fn given_an_empty_object_parameter_when_encoded_then_properties_is_an_empty_object() {
    let tools = tools_of(vec![tool(
        "t",
        "",
        vec![param(
            "o",
            "",
            true,
            ParameterKind::Object {
                parameters: parameters(vec![]),
            },
        )],
    )]);
    assert_eq!(
        tools[0]["input_schema"]["properties"]["o"],
        json!({"type": "object", "properties": {}})
    );
}

#[test]
fn given_a_tool_schema_when_encoded_then_it_has_no_additional_properties_and_no_strict() {
    let inner = parameters(vec![param("x", "", true, ParameterKind::String)]);
    let tools = tools_of(vec![tool(
        "t",
        "",
        vec![param(
            "o",
            "",
            true,
            ParameterKind::Object { parameters: inner },
        )],
    )]);
    let text = tools.to_string();
    assert!(!text.contains("additionalProperties"));
    assert!(!text.contains("strict"));
}

#[test]
fn given_several_tools_when_encoded_then_they_keep_the_order_of_the_request() {
    let tools = tools_of(vec![
        tool("zeta", "", vec![]),
        tool("alpha", "", vec![]),
        tool("mid", "", vec![]),
    ]);
    let names: Vec<_> = tools
        .as_array()
        .expect("array")
        .iter()
        .map(|tool| tool["name"].as_str().expect("name").to_owned())
        .collect();
    assert_eq!(names, ["zeta", "alpha", "mid"]);
}

#[test]
fn given_tools_when_encoded_then_no_tool_choice_is_sent() {
    let request = request_with(
        None,
        vec![user("hello")],
        vec![tool("t", "", vec![])],
        OutputMode::Plain,
    );
    assert!(body_of(&dialect(), &request).get("tool_choice").is_none());
}
