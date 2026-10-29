use br_llm_clients_core::{ParameterKind, Parameters};
use serde_json::{Value, json};

use crate::test_support::{
    dialect, encoded_body, hello, parameter, parameters, request_with, tool,
};
use crate::test_thread::user;

fn tools_of(list: Vec<br_llm_clients_core::ToolSpec>) -> Value {
    let request = request_with(
        None,
        vec![user("hello")],
        list,
        br_llm_clients_core::OutputMode::Plain,
    );
    encoded_body(&dialect(), &request)["tools"].clone()
}

fn function_of(list: Vec<br_llm_clients_core::Parameter>) -> Value {
    tools_of(vec![tool("lookup", "finds things", list)])[0]["function"].clone()
}

#[test]
fn given_a_tool_when_encoded_then_it_is_a_function_with_name_description_and_parameters() {
    let tools = tools_of(vec![tool(
        "lookup",
        "finds things",
        vec![parameter(
            "query",
            "what to find",
            true,
            ParameterKind::String,
        )],
    )]);
    assert_eq!(
        tools,
        json!([{"type": "function", "function": {
            "name": "lookup",
            "description": "finds things",
            "parameters": {
                "type": "object",
                "properties": {"query": {"type": "string", "description": "what to find"}},
                "required": ["query"]
            }
        }}])
    );
}

#[test]
fn given_an_empty_description_when_encoded_then_the_description_member_is_not_written() {
    let tools = tools_of(vec![tool("lookup", "", Vec::new())]);
    assert!(tools[0]["function"].get("description").is_none());
}

#[test]
fn given_a_tool_with_no_parameter_when_encoded_then_an_empty_object_schema_is_written() {
    let function = function_of(Vec::new());
    assert_eq!(
        function["parameters"],
        json!({"type": "object", "properties": {}})
    );
}

#[test]
fn given_each_scalar_kind_when_encoded_then_it_is_its_json_schema_type() {
    let function = function_of(vec![
        parameter("s", "", false, ParameterKind::String),
        parameter("n", "", false, ParameterKind::Number),
        parameter("i", "", false, ParameterKind::Integer),
        parameter("b", "", false, ParameterKind::Boolean),
    ]);
    assert_eq!(
        function["parameters"]["properties"],
        json!({
            "s": {"type": "string"},
            "n": {"type": "number"},
            "i": {"type": "integer"},
            "b": {"type": "boolean"}
        })
    );
}

#[test]
fn given_an_enum_when_encoded_then_it_is_a_string_with_its_values_in_order() {
    let kind = ParameterKind::Enum {
        values: vec!["red".to_owned(), "green".to_owned(), "blue".to_owned()],
    };
    let function = function_of(vec![parameter("color", "", true, kind)]);
    assert_eq!(
        function["parameters"]["properties"]["color"],
        json!({"type": "string", "enum": ["red", "green", "blue"]})
    );
}

#[test]
fn given_an_array_when_encoded_then_items_come_from_the_item_kind_without_a_description() {
    let kind = ParameterKind::Array {
        items: Box::new(ParameterKind::Integer),
    };
    let function = function_of(vec![parameter("ids", "the ids", true, kind)]);
    assert_eq!(
        function["parameters"]["properties"]["ids"],
        json!({"type": "array", "items": {"type": "integer"}, "description": "the ids"})
    );
}

#[test]
fn given_nested_arrays_and_objects_when_encoded_then_each_level_is_an_object_schema() {
    let inner = parameters(vec![
        parameter("x", "", true, ParameterKind::Number),
        parameter("y", "", false, ParameterKind::Number),
    ]);
    let kind = ParameterKind::Array {
        items: Box::new(ParameterKind::Object { parameters: inner }),
    };
    let function = function_of(vec![parameter("points", "", true, kind)]);
    assert_eq!(
        function["parameters"]["properties"]["points"],
        json!({"type": "array", "items": {
            "type": "object",
            "properties": {"x": {"type": "number"}, "y": {"type": "number"}},
            "required": ["x"]
        }})
    );
}

#[test]
fn given_an_object_parameter_with_no_inner_parameter_when_encoded_then_properties_stay_present() {
    let kind = ParameterKind::Object {
        parameters: Parameters::default(),
    };
    let function = function_of(vec![parameter("options", "free form", false, kind)]);
    assert_eq!(
        function["parameters"]["properties"]["options"],
        json!({"type": "object", "properties": {}, "description": "free form"})
    );
}

#[test]
fn given_only_optional_parameters_when_encoded_then_no_required_member_is_written() {
    let function = function_of(vec![parameter("a", "", false, ParameterKind::String)]);
    assert!(function["parameters"].get("required").is_none());
}

#[test]
fn given_required_parameters_when_encoded_then_required_lists_their_names_in_order() {
    let function = function_of(vec![
        parameter("zeta", "", true, ParameterKind::String),
        parameter("alpha", "", false, ParameterKind::String),
        parameter("mid", "", true, ParameterKind::String),
    ]);
    assert_eq!(function["parameters"]["required"], json!(["zeta", "mid"]));
}

#[test]
fn given_any_schema_when_encoded_then_additional_properties_and_strict_are_never_written() {
    let inner = ParameterKind::Object {
        parameters: parameters(vec![parameter("x", "", true, ParameterKind::String)]),
    };
    let tools = tools_of(vec![tool(
        "lookup",
        "d",
        vec![parameter("o", "", true, inner)],
    )]);
    let text = tools.to_string();
    assert!(!text.contains("additionalProperties"));
    assert!(!text.contains("strict"));
}

#[test]
fn given_several_tools_when_encoded_then_they_keep_the_order_of_the_request() {
    let tools = tools_of(vec![
        tool("zeta", "", Vec::new()),
        tool("alpha", "", Vec::new()),
        tool("mid", "", Vec::new()),
    ]);
    let names: Vec<&str> = tools
        .as_array()
        .expect("array")
        .iter()
        .filter_map(|tool| tool["function"]["name"].as_str())
        .collect();
    assert_eq!(names, ["zeta", "alpha", "mid"]);
}

#[test]
fn given_tools_when_encoded_then_no_tool_choice_and_no_parallel_flag_are_written() {
    let request = request_with(
        None,
        vec![user("hello")],
        vec![tool("lookup", "", Vec::new())],
        br_llm_clients_core::OutputMode::Plain,
    );
    let body = encoded_body(&dialect(), &request);
    assert!(body.get("tool_choice").is_none());
    assert!(body.get("parallel_tool_calls").is_none());
    assert!(encoded_body(&dialect(), &hello()).get("tools").is_none());
}
