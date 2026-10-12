use super::*;
use crate::build_error::BuildError;
use crate::test_support::{parameters, scalar};

fn objects_around_a_leaf(containers: usize) -> Parameters {
    let mut current = parameters(vec![scalar("leaf")]);
    for level in (0..containers).rev() {
        let wrapper = Parameter::new(
            format!("level{level}"),
            "",
            true,
            ParameterKind::Object {
                parameters: current,
            },
        )
        .unwrap();
        current = parameters(vec![wrapper]);
    }
    current
}

fn arrays_around_a_string(layers: usize) -> Parameters {
    let mut kind = ParameterKind::String;
    for _ in 0..layers {
        kind = ParameterKind::Array {
            items: Box::new(kind),
        };
    }
    parameters(vec![Parameter::new("a", "", true, kind).unwrap()])
}

#[test]
fn given_a_parameter_inside_32_objects_when_checked_then_accepted() {
    assert_eq!(check_depth(&objects_around_a_leaf(32)), Ok(()));
}

#[test]
fn given_a_parameter_inside_33_objects_when_checked_then_refused_naming_it() {
    assert_eq!(
        check_depth(&objects_around_a_leaf(33)),
        Err(BuildError::NestingTooDeep {
            parameter: "leaf".to_owned()
        })
    );
}

#[test]
fn given_32_nested_arrays_when_checked_then_accepted() {
    assert_eq!(check_depth(&arrays_around_a_string(32)), Ok(()));
}

#[test]
fn given_33_nested_arrays_when_checked_then_the_item_counts_and_the_declaring_parameter_is_named() {
    assert_eq!(
        check_depth(&arrays_around_a_string(33)),
        Err(BuildError::NestingTooDeep {
            parameter: "a".to_owned()
        })
    );
}

#[test]
fn given_an_empty_object_at_the_limit_when_checked_then_accepted() {
    let mut current = parameters(vec![
        Parameter::new(
            "empty",
            "",
            true,
            ParameterKind::Object {
                parameters: Parameters::default(),
            },
        )
        .unwrap(),
    ]);
    for level in (0..32).rev() {
        let wrapper = Parameter::new(
            format!("level{level}"),
            "",
            true,
            ParameterKind::Object {
                parameters: current,
            },
        )
        .unwrap();
        current = parameters(vec![wrapper]);
    }
    assert_eq!(check_depth(&current), Ok(()));
}

#[test]
fn given_two_too_deep_parameters_when_checked_then_the_first_in_declaration_order_is_named() {
    let deep_first = objects_around_a_leaf(33);
    let first = deep_first.iter().next().unwrap().clone();
    let second_leaf = Parameter::new(
        "second",
        "",
        true,
        ParameterKind::Object {
            parameters: parameters(vec![scalar("late_leaf")]),
        },
    )
    .unwrap();
    let list = parameters(vec![first, second_leaf]);
    assert_eq!(
        check_depth(&list),
        Err(BuildError::NestingTooDeep {
            parameter: "leaf".to_owned()
        })
    );
}

#[test]
fn given_a_shallower_sibling_declared_first_when_checked_then_the_deep_one_is_still_found() {
    let deep = objects_around_a_leaf(33).iter().next().unwrap().clone();
    let list = parameters(vec![scalar("shallow"), deep]);
    assert_eq!(
        check_depth(&list),
        Err(BuildError::NestingTooDeep {
            parameter: "leaf".to_owned()
        })
    );
}

fn mixed_containers_around_a_leaf(containers: usize) -> Parameters {
    let mut kind = ParameterKind::String;
    for level in 0..containers {
        kind = if level % 2 == 0 {
            ParameterKind::Array {
                items: Box::new(kind),
            }
        } else {
            ParameterKind::Object {
                parameters: parameters(vec![
                    Parameter::new(format!("o{level}"), "", true, kind).unwrap(),
                ]),
            }
        };
    }
    parameters(vec![Parameter::new("root", "", true, kind).unwrap()])
}

#[test]
fn given_32_containers_mixing_arrays_and_objects_when_checked_then_accepted() {
    assert_eq!(check_depth(&mixed_containers_around_a_leaf(32)), Ok(()));
}

#[test]
fn given_33_containers_mixing_arrays_and_objects_when_checked_then_the_deepest_declaring_parameter_is_named()
 {
    assert_eq!(
        check_depth(&mixed_containers_around_a_leaf(33)),
        Err(BuildError::NestingTooDeep {
            parameter: "o1".to_owned()
        })
    );
}

fn stored_array_kind(layers: usize) -> serde_json::Value {
    let mut kind = serde_json::json!({ "type": "string" });
    for _ in 0..layers {
        kind = serde_json::json!({ "type": "array", "items": kind });
    }
    kind
}

fn stored_parameter(kind: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "name": "a", "description": "", "required": true, "kind": kind })
}

#[test]
fn given_a_standalone_parameters_list_stored_40_deep_when_loaded_then_refused() {
    let stored = serde_json::json!([stored_parameter(stored_array_kind(40))]);
    assert!(serde_json::from_value::<Parameters>(stored).is_err());
}

#[test]
fn given_a_standalone_parameters_list_stored_at_the_limit_when_loaded_then_accepted() {
    let stored = serde_json::json!([stored_parameter(stored_array_kind(32))]);
    assert!(serde_json::from_value::<Parameters>(stored).is_ok());
}

#[test]
fn given_a_standalone_parameter_stored_one_level_too_deep_when_loaded_then_refused() {
    let at_limit = stored_parameter(stored_array_kind(32));
    let too_deep = stored_parameter(stored_array_kind(33));
    assert!(serde_json::from_value::<Parameter>(at_limit).is_ok());
    assert!(serde_json::from_value::<Parameter>(too_deep).is_err());
}

#[test]
fn given_a_standalone_kind_stored_one_level_too_deep_when_loaded_then_refused() {
    assert!(serde_json::from_value::<ParameterKind>(stored_array_kind(32)).is_ok());
    assert!(serde_json::from_value::<ParameterKind>(stored_array_kind(33)).is_err());
}
