use br_llm_clients_core::{OutputMode, ParameterKind};
use serde_json::{Value, json};

use crate::test_requests::{body_of, parameter, parameters, request_with, tool, user_text};

fn tools_of(specs: Vec<br_llm_clients_core::ToolSpec>) -> Vec<Value> {
    let request = request_with(None, vec![user_text("go")], specs, OutputMode::Plain);
    body_of(&request)["tools"]
        .as_array()
        .expect("tools")
        .clone()
}

fn one(spec: br_llm_clients_core::ToolSpec) -> Value {
    tools_of(vec![spec]).remove(0)
}

fn string(name: &str, required: bool) -> br_llm_clients_core::Parameter {
    parameter(name, "", required, ParameterKind::String)
}

#[test]
fn given_a_tool_with_a_description_when_encoded_then_it_is_a_function_with_all_fields() {
    let encoded = one(tool(
        "get_weather",
        "weather of a city",
        vec![
            parameter("city", "the city", true, ParameterKind::String),
            parameter("days", "how many", true, ParameterKind::Integer),
        ],
    ));
    assert_eq!(
        encoded,
        json!({
            "type": "function",
            "name": "get_weather",
            "description": "weather of a city",
            "parameters": {
                "type": "object",
                "properties": {
                    "city": { "type": "string", "description": "the city" },
                    "days": { "type": "integer", "description": "how many" }
                },
                "required": ["city", "days"],
                "additionalProperties": false
            },
            "strict": true
        })
    );
}

#[test]
fn given_an_empty_description_when_encoded_then_the_description_is_not_sent() {
    let encoded = one(tool("ping", "", Vec::new()));
    assert!(encoded.get("description").is_none());
}

#[test]
fn given_a_tool_without_parameters_when_encoded_then_properties_is_empty_and_strict_is_true() {
    let encoded = one(tool("ping", "", Vec::new()));
    assert_eq!(
        encoded["parameters"],
        json!({
            "type": "object",
            "properties": {},
            "required": [],
            "additionalProperties": false
        })
    );
    assert_eq!(encoded["strict"], json!(true));
}

#[test]
fn given_several_tools_when_encoded_then_they_keep_the_order_of_the_request() {
    let names: Vec<String> = tools_of(vec![
        tool("zeta", "", Vec::new()),
        tool("alpha", "", Vec::new()),
        tool("mid", "", Vec::new()),
    ])
    .iter()
    .map(|spec| spec["name"].as_str().expect("name").to_owned())
    .collect();
    assert_eq!(names, ["zeta", "alpha", "mid"]);
}

#[test]
fn given_every_scalar_kind_when_encoded_then_each_has_its_type() {
    let encoded = one(tool(
        "t",
        "",
        vec![
            parameter("s", "", true, ParameterKind::String),
            parameter("n", "", true, ParameterKind::Number),
            parameter("i", "", true, ParameterKind::Integer),
            parameter("b", "", true, ParameterKind::Boolean),
        ],
    ));
    let properties = &encoded["parameters"]["properties"];
    assert_eq!(properties["s"], json!({ "type": "string" }));
    assert_eq!(properties["n"], json!({ "type": "number" }));
    assert_eq!(properties["i"], json!({ "type": "integer" }));
    assert_eq!(properties["b"], json!({ "type": "boolean" }));
}

#[test]
fn given_an_enum_when_encoded_then_it_is_a_string_with_its_values_in_order() {
    let kind = ParameterKind::Enum {
        values: vec!["red".to_owned(), "green".to_owned(), "blue".to_owned()],
    };
    let encoded = one(tool("t", "", vec![parameter("color", "", true, kind)]));
    assert_eq!(
        encoded["parameters"]["properties"]["color"],
        json!({ "type": "string", "enum": ["red", "green", "blue"] })
    );
}

#[test]
fn given_an_array_of_objects_when_encoded_then_items_is_made_from_the_item_kind() {
    let item = ParameterKind::Object {
        parameters: parameters(vec![string("name", true)]),
    };
    let kind = ParameterKind::Array {
        items: Box::new(item),
    };
    let encoded = one(tool("t", "", vec![parameter("people", "", true, kind)]));
    assert_eq!(
        encoded["parameters"]["properties"]["people"],
        json!({
            "type": "array",
            "items": {
                "type": "object",
                "properties": { "name": { "type": "string" } },
                "required": ["name"],
                "additionalProperties": false
            }
        })
    );
}

#[test]
fn given_a_nested_object_when_encoded_then_it_is_made_the_same_way_with_its_description() {
    let kind = ParameterKind::Object {
        parameters: parameters(vec![
            parameter("street", "the street", true, ParameterKind::String),
            string("zip", false),
        ]),
    };
    let encoded = one(tool(
        "t",
        "",
        vec![parameter("address", "where", true, kind)],
    ));
    assert_eq!(
        encoded["parameters"]["properties"]["address"],
        json!({
            "type": "object",
            "description": "where",
            "properties": {
                "street": { "type": "string", "description": "the street" },
                "zip": { "type": "string" }
            },
            "required": ["street"],
            "additionalProperties": false
        })
    );
}

#[test]
fn given_optional_parameters_when_encoded_then_required_lists_only_the_required_in_declaration_order()
 {
    let encoded = one(tool(
        "t",
        "",
        vec![
            string("z_first", true),
            string("a_optional", false),
            string("m_last", true),
        ],
    ));
    assert_eq!(
        encoded["parameters"]["required"],
        json!(["z_first", "m_last"])
    );
    assert_eq!(encoded["strict"], json!(false));
}

#[test]
fn given_optional_parameters_when_encoded_then_no_null_type_emulates_them() {
    let encoded = one(tool("t", "", vec![string("a", false)]));
    let text = encoded.to_string();
    assert!(!text.contains("\"null\""));
    assert_eq!(
        encoded["parameters"]["properties"]["a"],
        json!({ "type": "string" })
    );
}

#[test]
fn given_every_parameter_required_at_the_top_when_encoded_then_strict_is_true() {
    let encoded = one(tool("t", "", vec![string("a", true), string("b", true)]));
    assert_eq!(encoded["strict"], json!(true));
}

#[test]
fn given_an_optional_parameter_in_a_nested_object_when_encoded_then_strict_is_false() {
    let kind = ParameterKind::Object {
        parameters: parameters(vec![string("inner", false)]),
    };
    let encoded = one(tool("t", "", vec![parameter("outer", "", true, kind)]));
    assert_eq!(encoded["strict"], json!(false));
}

#[test]
fn given_an_optional_parameter_in_an_object_inside_an_array_when_encoded_then_strict_is_false() {
    let kind = ParameterKind::Array {
        items: Box::new(ParameterKind::Object {
            parameters: parameters(vec![string("inner", false)]),
        }),
    };
    let encoded = one(tool("t", "", vec![parameter("list", "", true, kind)]));
    assert_eq!(encoded["strict"], json!(false));
}

#[test]
fn given_an_optional_parameter_two_levels_down_when_encoded_then_strict_is_false() {
    let deep = ParameterKind::Object {
        parameters: parameters(vec![string("leaf", false)]),
    };
    let middle = ParameterKind::Object {
        parameters: parameters(vec![parameter("deep", "", true, deep)]),
    };
    let encoded = one(tool("t", "", vec![parameter("top", "", true, middle)]));
    assert_eq!(encoded["strict"], json!(false));
}

#[test]
fn given_all_required_at_every_level_when_encoded_then_strict_is_true() {
    let deep = ParameterKind::Object {
        parameters: parameters(vec![string("leaf", true)]),
    };
    let list = ParameterKind::Array {
        items: Box::new(deep),
    };
    let encoded = one(tool("t", "", vec![parameter("top", "", true, list)]));
    assert_eq!(encoded["strict"], json!(true));
}

#[test]
fn given_a_tool_when_encoded_then_every_object_level_forbids_other_properties() {
    let kind = ParameterKind::Object {
        parameters: parameters(vec![string("x", true)]),
    };
    let encoded = one(tool("t", "", vec![parameter("o", "", true, kind)]));
    assert_eq!(encoded["parameters"]["additionalProperties"], json!(false));
    assert_eq!(
        encoded["parameters"]["properties"]["o"]["additionalProperties"],
        json!(false)
    );
}

#[test]
fn given_many_tools_and_deep_schemas_when_encoded_then_the_dialect_counts_nothing_and_refuses_nothing()
 {
    let mut kind = ParameterKind::String;
    for _ in 0..20 {
        kind = ParameterKind::Array {
            items: Box::new(kind),
        };
    }
    let specs: Vec<_> = (0..200)
        .map(|n| {
            tool(
                &format!("tool_{n}"),
                "",
                vec![parameter("p", "", true, kind.clone())],
            )
        })
        .collect();
    assert_eq!(tools_of(specs).len(), 200);
}
