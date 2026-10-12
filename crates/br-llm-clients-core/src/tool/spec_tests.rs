use br_llm_messages::ToolName;

use super::*;
use crate::build_error::BuildError;
use crate::test_support::{parameters, scalar};

#[test]
fn given_a_tool_without_parameters_when_built_then_accepted() {
    let spec = ToolSpec::new(ToolName::new("ping").unwrap(), "", Parameters::default());
    assert!(spec.is_ok());
}

#[test]
fn given_a_too_deep_declaration_when_the_tool_is_built_then_refused() {
    let mut kind = ParameterKind::String;
    for _ in 0..33 {
        kind = ParameterKind::Array {
            items: Box::new(kind),
        };
    }
    let list = parameters(vec![Parameter::new("a", "", true, kind).unwrap()]);
    assert_eq!(
        ToolSpec::new(ToolName::new("t").unwrap(), "", list),
        Err(BuildError::NestingTooDeep {
            parameter: "a".to_owned()
        })
    );
}

#[test]
fn given_a_tool_when_round_tripped_then_identical() {
    let spec = ToolSpec::new(
        ToolName::new("search").unwrap(),
        "finds things",
        parameters(vec![scalar("query")]),
    )
    .unwrap();
    let json = serde_json::to_value(&spec).unwrap();
    assert_eq!(serde_json::from_value::<ToolSpec>(json).unwrap(), spec);
}

#[test]
fn given_a_stored_tool_nested_too_deep_when_loaded_then_refused() {
    let mut kind = serde_json::json!({ "type": "string" });
    for _ in 0..33 {
        kind = serde_json::json!({ "type": "array", "items": kind });
    }
    let stored = serde_json::json!({
        "name": "t",
        "description": "",
        "parameters": [ { "name": "a", "description": "", "required": true, "kind": kind } ]
    });
    assert!(serde_json::from_value::<ToolSpec>(stored).is_err());
}

#[test]
fn given_a_stored_tool_with_an_empty_name_when_loaded_then_refused() {
    let stored = serde_json::json!({ "name": "", "description": "", "parameters": [] });
    assert!(serde_json::from_value::<ToolSpec>(stored).is_err());
}
