use serde_json::{Map, Value, json};

use crate::{OpenAiResponses, Options, OptionsError, PromptCacheMode, ServiceTier};

const RESERVED: [&str; 13] = [
    "model",
    "stream",
    "store",
    "instructions",
    "input",
    "max_output_tokens",
    "tools",
    "text",
    "reasoning",
    "include",
    "prompt_cache_key",
    "service_tier",
    "prompt_cache_options",
];

fn members(pairs: &[(&str, Value)]) -> Map<String, Value> {
    pairs
        .iter()
        .map(|(name, value)| ((*name).to_owned(), value.clone()))
        .collect()
}

#[test]
fn given_each_reserved_member_when_set_in_extra_body_then_it_is_refused_and_named() {
    for name in RESERVED {
        let error = Options::default()
            .with_extra_body(members(&[(name, json!("SECRET-VALUE"))]))
            .expect_err("a reserved member");
        assert_eq!(
            error,
            OptionsError::ReservedExtraBodyMember {
                member: name.to_owned()
            }
        );
        let shown = error.to_string();
        assert!(shown.contains(name), "{name}");
        assert!(!shown.contains("SECRET-VALUE"), "{name}");
        assert!(!format!("{error:?}").contains("SECRET-VALUE"), "{name}");
    }
}

#[test]
fn given_a_reserved_member_next_to_free_ones_when_set_then_it_is_still_refused() {
    let error = Options::default()
        .with_extra_body(members(&[("temperature", json!(1)), ("model", json!("x"))]))
        .expect_err("a reserved member");
    assert_eq!(
        error,
        OptionsError::ReservedExtraBodyMember {
            member: "model".to_owned()
        }
    );
}

#[test]
fn given_several_reserved_members_when_set_then_one_of_them_is_named() {
    let error = Options::default()
        .with_extra_body(members(&[("model", json!(1)), ("tools", json!(2))]))
        .expect_err("a reserved member");
    let OptionsError::ReservedExtraBodyMember { member: name } = error;
    assert!(name == "model" || name == "tools");
}

#[test]
fn given_an_extra_body_when_read_then_the_object_is_given() {
    let object = members(&[("temperature", json!(0.1))]);
    let options = Options::default()
        .with_extra_body(object.clone())
        .expect("allowed");
    assert_eq!(options.extra_body(), &object);
    assert!(Options::default().extra_body().is_empty());
}

#[test]
fn given_a_cache_key_and_an_extra_body_when_debug_printed_then_neither_the_text_nor_a_value_shows()
{
    let options = Options {
        prompt_cache_key: Some("KEY-TEXT-9".to_owned()),
        ..Options::default()
    }
    .with_extra_body(members(&[
        ("temperature", json!("VALUE-TEXT-7")),
        ("metadata", json!({ "k": "VALUE-TEXT-8" })),
    ]))
    .expect("allowed");
    for shown in [format!("{options:?}"), format!("{options:#?}")] {
        assert!(!shown.contains("KEY-TEXT-9"));
        assert!(!shown.contains("VALUE-TEXT-7"));
        assert!(!shown.contains("VALUE-TEXT-8"));
        assert!(shown.contains("temperature"));
        assert!(shown.contains("metadata"));
        assert!(shown.contains("prompt_cache_key"));
    }
}

#[test]
fn given_a_cache_key_set_or_not_when_debug_printed_then_only_the_presence_shows() {
    let unset = format!("{:?}", Options::default());
    let set = format!(
        "{:?}",
        Options {
            prompt_cache_key: Some("k".to_owned()),
            ..Options::default()
        }
    );
    assert_ne!(unset, set);
    assert!(unset.contains("unset"));
    assert!(!set.contains("unset"));
}

#[test]
fn given_the_dialect_when_debug_printed_then_it_shows_the_options_without_secrets() {
    let options = Options {
        prompt_cache_key: Some("KEY-TEXT-9".to_owned()),
        ..Options::default()
    };
    let dialect =
        crate::test_requests::dialect_with(br_llm_clients_core::ReasoningLevel::Off, options);
    assert!(!format!("{dialect:?}").contains("KEY-TEXT-9"));
}

#[test]
fn given_an_options_with_every_field_set_when_the_dialect_is_built_then_it_is_send_sync_and_clone()
{
    fn assert_send_sync<T: Send + Sync>(_: &T) {}
    let options = Options {
        prompt_cache_key: Some("k".to_owned()),
        service_tier: Some(ServiceTier::Auto),
        cache_mode: Some(PromptCacheMode::Implicit),
        ..Options::default()
    }
    .with_extra_body(members(&[("temperature", json!(0.3))]))
    .expect("allowed");
    let settings = crate::test_requests::settings(
        br_llm_clients_core::ReasoningLevel::Low,
        br_llm_clients_core::CacheRequirement::Required,
    );
    let dialect = OpenAiResponses::new(&settings, options.clone());
    assert_send_sync(&dialect);
    assert_send_sync(&options);
    let copy = dialect.clone();
    let request = crate::test_requests::hello();
    let original = crate::test_requests::encode_body(&dialect, &request).expect("encoded");
    assert_eq!(original["temperature"], json!(0.3));
    assert_eq!(
        crate::test_requests::encode_body(&copy, &request).expect("encoded"),
        original
    );
}

#[test]
fn given_a_cloned_options_when_compared_then_the_extra_body_object_is_copied() {
    let options = Options::default()
        .with_extra_body(members(&[("temperature", json!(0.3))]))
        .expect("allowed");
    let clone = options.clone();
    assert_eq!(clone, options);
    assert_eq!(clone.extra_body(), options.extra_body());
    assert_ne!(options, Options::default());
}

#[test]
fn given_a_request_with_every_dialect_member_when_encoded_then_each_body_key_is_reserved() {
    use br_llm_clients_core::{OutputMode, ParameterKind, ReasoningLevel};

    use crate::test_requests::{
        dialect_with, encode_body, parameter, request_with, shape, tool, user_text,
    };

    let options = Options {
        prompt_cache_key: Some("k".to_owned()),
        service_tier: Some(ServiceTier::Auto),
        cache_mode: Some(PromptCacheMode::Implicit),
        ..Options::default()
    };
    let request = request_with(
        Some("be brief"),
        vec![user_text("hi")],
        vec![tool(
            "lookup",
            "",
            vec![parameter("q", "", true, ParameterKind::String)],
        )],
        OutputMode::Structured(shape(
            "answer",
            None,
            vec![parameter("value", "", true, ParameterKind::String)],
        )),
    );
    let body = encode_body(&dialect_with(ReasoningLevel::High, options), &request).expect("body");
    let keys: Vec<&String> = body.as_object().expect("object").keys().collect();
    assert_eq!(keys.len(), RESERVED.len());
    for key in keys {
        let refused = Options::default().with_extra_body(members(&[(key, json!(0))]));
        assert!(refused.is_err(), "{key}");
    }
}
