use br_llm_clients_core::{CacheRequirement, ReasoningLevel};
use serde_json::{Map, Value, json};

use crate::test_fixtures::*;
use crate::test_shapes::every_request_shape;
use crate::{AnthropicMessages, MessagesOptions, ModelPlacement, OptionsError, VersionPlacement};

pub(crate) const RESERVED: [&str; 10] = [
    "model",
    "max_tokens",
    "stream",
    "messages",
    "system",
    "tools",
    "thinking",
    "output_config",
    "cache_control",
    "anthropic_version",
];

fn object(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        other => panic!("not an object: {other}"),
    }
}

fn extra(value: Value) -> MessagesOptions {
    MessagesOptions::new()
        .with_extra_body(object(value))
        .expect("accepted")
}

fn body_with(options: MessagesOptions) -> Value {
    body_of(&dialect_options(options), &request(vec![user("hello")]))
}

fn default_body() -> Value {
    body_of(&dialect(), &request(vec![user("hello")]))
}

fn without(mut body: Value, member: &str) -> Value {
    body.as_object_mut().expect("object").remove(member);
    body
}

#[test]
fn given_the_model_placement_omitted_when_encoded_then_only_the_model_member_is_absent() {
    let body = body_with(MessagesOptions::new().with_model_placement(ModelPlacement::Omitted));
    assert!(body.get("model").is_none());
    assert_eq!(body, without(default_body(), "model"));
}

#[test]
fn given_the_model_placement_body_when_encoded_then_the_member_is_the_model_of_the_settings() {
    let body = body_with(
        MessagesOptions::new()
            .with_model_placement(ModelPlacement::Omitted)
            .with_model_placement(ModelPlacement::Body),
    );
    assert_eq!(body["model"], json!(MODEL));
    assert_eq!(body, default_body());
}

#[test]
fn given_the_version_placement_body_when_encoded_then_the_member_is_set_and_the_header_is_gone() {
    let options = MessagesOptions::new().with_version_placement(vertex());
    let encoded =
        encode(&dialect_options(options), &request(vec![user("hello")])).expect("encoded");
    let body: Value = serde_json::from_slice(encoded.body()).expect("json");
    assert_eq!(body["anthropic_version"], json!("vertex-2023-10-16"));
    assert_eq!(without(body, "anthropic_version"), default_body());
    assert_eq!(
        encoded.headers(),
        &[("content-type".to_owned(), "application/json".to_owned())]
    );
}

#[test]
fn given_the_version_placement_header_when_encoded_then_the_header_is_set_and_the_member_is_gone() {
    let options = MessagesOptions::new()
        .with_version_placement(vertex())
        .with_version_placement(VersionPlacement::Header);
    let encoded =
        encode(&dialect_options(options), &request(vec![user("hello")])).expect("encoded");
    let body: Value = serde_json::from_slice(encoded.body()).expect("json");
    assert!(body.get("anthropic_version").is_none());
    assert_eq!(body, default_body());
    assert!(
        encoded
            .headers()
            .contains(&("anthropic-version".to_owned(), "2023-06-01".to_owned()))
    );
}

#[test]
fn given_a_blank_version_text_when_encoded_then_it_is_written_as_it_is() {
    for text in ["", "   "] {
        let options =
            MessagesOptions::new().with_version_placement(VersionPlacement::Body(text.to_owned()));
        assert_eq!(body_with(options)["anthropic_version"], json!(text));
    }
}

#[test]
fn given_an_extra_body_when_encoded_then_its_members_are_at_the_top_level_with_their_values() {
    let body = body_with(extra(
        json!({"metadata": {"user_id": "u1"}, "temperature": 0.5}),
    ));
    assert_eq!(body["metadata"], json!({"user_id": "u1"}));
    assert_eq!(body["temperature"], json!(0.5));
    assert_eq!(
        without(without(body, "metadata"), "temperature"),
        default_body()
    );
}

#[test]
fn given_a_nested_extra_body_when_encoded_then_the_value_is_written_whole() {
    let value = json!({"a": {"b": [1, {"c": null}], "d": "e"}});
    let body = body_with(extra(json!({"custom": value.clone()})));
    assert_eq!(body["custom"], value);
}

#[test]
fn given_two_extra_bodies_when_set_then_the_second_replaces_the_first() {
    let options = extra(json!({"metadata": {"user_id": "u1"}}))
        .with_extra_body(object(json!({"temperature": 0.5})))
        .expect("accepted");
    let body = body_with(options);
    assert!(body.get("metadata").is_none());
    assert_eq!(body["temperature"], json!(0.5));
}

#[test]
fn given_an_empty_extra_body_when_encoded_then_the_body_is_the_default_body() {
    assert_eq!(body_with(extra(json!({}))), default_body());
    let replaced = extra(json!({"temperature": 0.5}))
        .with_extra_body(Map::new())
        .expect("accepted");
    assert_eq!(body_with(replaced), default_body());
}

#[test]
fn given_an_extra_body_when_debug_is_shown_then_the_names_appear_and_no_value_does() {
    let text = format!(
        "{:?}",
        extra(json!({"metadata": {"user_id": "secret-value-1"}}))
    );
    assert!(text.contains("metadata"));
    assert!(!text.contains("secret-value-1"));
    assert!(!text.contains("user_id"));
}

#[test]
fn given_the_default_options_when_debug_is_shown_then_the_member_list_is_empty() {
    let text = format!("{:?}", MessagesOptions::new());
    assert!(text.contains("extra_body_members: []"), "{text}");
    for name in ["metadata", "temperature", "user_id"] {
        assert!(!text.contains(name), "{name} appeared in {text}");
    }
}

#[test]
fn given_each_reserved_name_when_set_as_extra_body_then_it_is_refused_with_that_name_whatever_the_placements()
 {
    let placements = [
        MessagesOptions::new(),
        MessagesOptions::new().with_model_placement(ModelPlacement::Omitted),
        MessagesOptions::new().with_version_placement(vertex()),
        MessagesOptions::new()
            .with_model_placement(ModelPlacement::Omitted)
            .with_version_placement(vertex()),
    ];
    for name in RESERVED {
        for options in &placements {
            let mut members = Map::new();
            members.insert(name.to_owned(), json!("secret-value-2"));
            let error = options
                .clone()
                .with_extra_body(members)
                .expect_err("refused");
            assert_eq!(
                error,
                OptionsError::ReservedExtraBodyMember {
                    member: name.to_owned()
                }
            );
            let text = error.to_string();
            assert!(text.contains(name));
            assert!(!text.contains("secret-value-2"));
        }
    }
}

#[test]
fn given_several_reserved_names_when_set_then_one_of_them_is_named() {
    let error = MessagesOptions::new()
        .with_extra_body(object(json!({"system": 1, "tools": 2, "metadata": 3})))
        .expect_err("refused");
    let OptionsError::ReservedExtraBodyMember { member } = error;
    assert!(["system", "tools"].contains(&member.as_str()));
}

#[test]
fn given_all_options_set_when_encoded_twice_then_the_bytes_are_equal_and_the_request_is_unchanged()
{
    let options = extra(json!({"metadata": {"user_id": "u1"}, "temperature": 0.5}))
        .with_model_placement(ModelPlacement::Omitted)
        .with_version_placement(vertex());
    let dialect = AnthropicMessages::with_options(
        &settings(ReasoningLevel::High, CacheRequirement::Required, 2048),
        options,
    );
    for shape in every_request_shape() {
        let before = shape.clone();
        let first = encode(&dialect, &shape).expect("first");
        let second = encode(&dialect, &shape).expect("second");
        assert_eq!(first, second);
        assert_eq!(shape, before);
    }
}

#[test]
fn given_a_serde_json_map_when_passed_to_with_extra_body_then_it_needs_no_conversion() {
    let members: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
    let reexported: crate::Map<String, crate::Value> = members;
    let again: serde_json::Map<String, serde_json::Value> = reexported.clone();
    assert!(
        MessagesOptions::new()
            .with_extra_body(reexported)
            .and_then(|options| options.with_extra_body(again))
            .is_ok()
    );
}

#[test]
fn given_the_vertex_options_when_encoded_then_the_model_is_absent_the_version_is_in_the_body_and_the_rest_is_default()
 {
    let options = extra(json!({"metadata": {"user_id": "u1"}}))
        .with_model_placement(ModelPlacement::Omitted)
        .with_version_placement(vertex());
    let encoded =
        encode(&dialect_options(options), &request(vec![user("hello")])).expect("encoded");
    let body: Value = serde_json::from_slice(encoded.body()).expect("json");
    assert!(body.get("model").is_none());
    assert_eq!(body["anthropic_version"], json!("vertex-2023-10-16"));
    assert_eq!(body["metadata"], json!({"user_id": "u1"}));
    let rest = without(
        without(without(body, "anthropic_version"), "metadata"),
        "model",
    );
    assert_eq!(rest, without(default_body(), "model"));
    assert_eq!(
        encoded.headers(),
        &[("content-type".to_owned(), "application/json".to_owned())]
    );
}
