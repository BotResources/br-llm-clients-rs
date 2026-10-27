use super::*;
use crate::build_error::BuildError;
use crate::test_support::{parameters, scalar};

#[test]
fn given_a_blank_name_when_built_then_refused() {
    for name in ["", "   ", "\n\t"] {
        assert_eq!(
            Parameter::new(name, "", true, ParameterKind::String),
            Err(BuildError::BlankParameterName)
        );
    }
}

#[test]
fn given_a_name_with_surrounding_space_when_built_then_kept_as_given() {
    let parameter = Parameter::new(" city ", "", true, ParameterKind::String).unwrap();
    assert_eq!(parameter.name(), " city ");
}

#[test]
fn given_an_empty_description_when_built_then_accepted() {
    assert!(Parameter::new("city", "", false, ParameterKind::Integer).is_ok());
}

#[test]
fn given_an_enum_without_value_when_built_then_refused_naming_the_parameter() {
    let kind = ParameterKind::Enum { values: vec![] };
    assert_eq!(
        Parameter::new("unit", "", true, kind),
        Err(BuildError::EmptyEnum {
            parameter: "unit".to_owned()
        })
    );
}

#[test]
fn given_an_enum_with_a_blank_value_when_built_then_refused_naming_the_parameter() {
    let kind = ParameterKind::Enum {
        values: vec!["c".to_owned(), "  ".to_owned()],
    };
    assert_eq!(
        Parameter::new("unit", "", true, kind),
        Err(BuildError::BlankEnumValue {
            parameter: "unit".to_owned()
        })
    );
}

#[test]
fn given_an_enum_with_a_repeated_value_when_built_then_refused_naming_parameter_and_value() {
    let kind = ParameterKind::Enum {
        values: vec!["c".to_owned(), "f".to_owned(), "c".to_owned()],
    };
    assert_eq!(
        Parameter::new("unit", "", true, kind),
        Err(BuildError::DuplicateEnumValue {
            parameter: "unit".to_owned(),
            value: "c".to_owned()
        })
    );
}

#[test]
fn given_enum_values_differing_only_by_case_or_space_when_built_then_accepted() {
    let kind = ParameterKind::Enum {
        values: vec!["c".to_owned(), "C".to_owned(), "c ".to_owned()],
    };
    assert!(Parameter::new("unit", "", true, kind).is_ok());
}

#[test]
fn given_an_array_of_an_invalid_enum_when_built_then_refused_naming_the_parameter() {
    let kind = ParameterKind::Array {
        items: Box::new(ParameterKind::Enum { values: vec![] }),
    };
    assert_eq!(
        Parameter::new("units", "", true, kind),
        Err(BuildError::EmptyEnum {
            parameter: "units".to_owned()
        })
    );
}

#[test]
fn given_two_parameters_with_one_name_when_listed_then_refused_naming_it() {
    assert_eq!(
        Parameters::new(vec![scalar("a"), scalar("b"), scalar("a")]),
        Err(BuildError::DuplicateParameterName {
            parameter: "a".to_owned()
        })
    );
}

#[test]
fn given_the_same_name_in_two_different_lists_when_built_then_accepted() {
    let inner = parameters(vec![scalar("a")]);
    let outer = Parameter::new("o", "", true, ParameterKind::Object { parameters: inner });
    assert!(Parameters::new(vec![scalar("a"), outer.unwrap()]).is_ok());
}

#[test]
fn given_an_empty_list_when_built_then_accepted() {
    assert!(Parameters::new(vec![]).unwrap().is_empty());
}

#[test]
fn given_a_nested_parameter_when_round_tripped_then_identical() {
    let item = ParameterKind::Object {
        parameters: parameters(vec![scalar("x")]),
    };
    let parameter = Parameter::new(
        "points",
        "a list",
        false,
        ParameterKind::Array {
            items: Box::new(item),
        },
    )
    .unwrap();
    let json = serde_json::to_value(&parameter).unwrap();
    assert_eq!(
        serde_json::from_value::<Parameter>(json).unwrap(),
        parameter
    );
}

#[test]
fn given_stored_parameters_breaking_a_rule_when_loaded_then_refused() {
    let blank_name = serde_json::json!({
        "name": " ", "description": "", "required": true, "kind": { "type": "string" }
    });
    let empty_enum = serde_json::json!({
        "name": "u", "description": "", "required": true,
        "kind": { "type": "enum", "values": [] }
    });
    let blank_value = serde_json::json!({
        "name": "u", "description": "", "required": true,
        "kind": { "type": "enum", "values": [" "] }
    });
    let repeated_value = serde_json::json!({
        "name": "u", "description": "", "required": true,
        "kind": { "type": "enum", "values": ["a", "a"] }
    });
    for stored in [blank_name, empty_enum, blank_value, repeated_value] {
        assert!(serde_json::from_value::<Parameter>(stored).is_err());
    }
}

#[test]
fn given_stored_duplicate_parameter_names_when_loaded_then_refused() {
    let stored = serde_json::json!([
        { "name": "a", "description": "", "required": true, "kind": { "type": "string" } },
        { "name": "a", "description": "", "required": false, "kind": { "type": "number" } }
    ]);
    assert!(serde_json::from_value::<Parameters>(stored).is_err());
}

#[test]
fn given_a_kind_unknown_to_this_version_when_loaded_then_refused() {
    let stored = serde_json::json!({
        "name": "a", "description": "", "required": true, "kind": { "type": "date" }
    });
    assert!(serde_json::from_value::<Parameter>(stored).is_err());
}

#[test]
fn given_a_stored_parameter_with_an_unknown_field_when_loaded_then_refused() {
    let stored = serde_json::json!({
        "name": "a", "description": "", "required": true,
        "kind": { "type": "string" }, "default": "x"
    });
    assert!(serde_json::from_value::<Parameter>(stored).is_err());
}

#[test]
fn given_a_standalone_kind_stored_breaking_an_enum_rule_when_loaded_then_refused() {
    let empty = serde_json::json!({ "type": "enum", "values": [] });
    let blank = serde_json::json!({ "type": "enum", "values": [" "] });
    let repeated = serde_json::json!({ "type": "enum", "values": ["a", "a"] });
    let nested = serde_json::json!({ "type": "array", "items": empty.clone() });
    for stored in [empty, blank, repeated, nested] {
        assert!(serde_json::from_value::<ParameterKind>(stored).is_err());
    }
}

#[test]
fn given_a_valid_standalone_kind_when_round_tripped_then_identical() {
    let kind = ParameterKind::Array {
        items: Box::new(ParameterKind::Enum {
            values: vec!["a".to_owned(), "b".to_owned()],
        }),
    };
    let json = serde_json::to_value(&kind).unwrap();
    assert_eq!(serde_json::from_value::<ParameterKind>(json).unwrap(), kind);
}

#[test]
fn given_an_object_kind_stored_with_a_blank_inner_name_when_loaded_then_refused() {
    let stored = serde_json::json!({
        "type": "object",
        "parameters": [
            { "name": " ", "description": "", "required": true, "kind": { "type": "string" } }
        ]
    });
    assert!(serde_json::from_value::<ParameterKind>(stored).is_err());
}

#[test]
fn given_a_stored_scalar_kind_with_an_unknown_key_when_loaded_then_refused() {
    for kind in ["string", "number", "integer", "boolean"] {
        let plain = serde_json::json!({ "type": kind });
        assert!(
            serde_json::from_value::<ParameterKind>(plain).is_ok(),
            "{kind}"
        );
        let extra = serde_json::json!({ "type": kind, "format": "date" });
        assert!(
            serde_json::from_value::<ParameterKind>(extra).is_err(),
            "{kind}"
        );
    }
}
