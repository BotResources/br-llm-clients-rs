use br_llm_clients_core::{CacheRequirement, ReasoningLevel};
use serde_json::{Map, Value, json};

use crate::test_requests::{dialect_with, encode_body, hello, settings};
use crate::{OpenAiResponses, Options, PromptCacheMode, ServiceTier};

fn base_body() -> Value {
    json!({
        "model": "gpt-5.1",
        "stream": true,
        "store": false,
        "max_output_tokens": 4096,
        "input": [{
            "type": "message",
            "role": "user",
            "content": [{ "type": "input_text", "text": "hello" }]
        }],
        "reasoning": { "effort": "none" }
    })
}

fn body_with(options: Options) -> Value {
    encode_body(&dialect_with(ReasoningLevel::Off, options), &hello()).expect("encoded")
}

fn object(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        other => panic!("{other}"),
    }
}

fn extra(value: Value) -> Options {
    Options::default()
        .with_extra_body(object(value))
        .expect("allowed members")
}

fn with_members(extra_members: Value) -> Value {
    let mut body = base_body();
    for (name, value) in object(extra_members) {
        body[name] = value;
    }
    body
}

#[test]
fn given_no_new_option_when_encoded_then_the_body_is_the_body_of_0_1() {
    assert_eq!(body_with(Options::default()), base_body());
}

#[test]
fn given_a_prompt_cache_key_when_encoded_then_it_goes_into_the_body_as_it_is() {
    let options = Options {
        prompt_cache_key: Some("session-42".to_owned()),
        ..Options::default()
    };
    assert_eq!(
        body_with(options),
        with_members(json!({ "prompt_cache_key": "session-42" }))
    );
}

#[test]
fn given_an_empty_prompt_cache_key_when_encoded_then_it_is_sent_unchecked() {
    let options = Options {
        prompt_cache_key: Some(String::new()),
        ..Options::default()
    };
    assert_eq!(body_with(options)["prompt_cache_key"], json!(""));
}

#[test]
fn given_a_service_tier_when_encoded_then_it_goes_into_the_body() {
    let options = Options {
        service_tier: Some(ServiceTier::Flex),
        ..Options::default()
    };
    assert_eq!(
        body_with(options),
        with_members(json!({ "service_tier": "flex" }))
    );
}

#[test]
fn given_every_service_tier_when_encoded_then_the_wire_values_follow_the_table() {
    let tiers = [
        (ServiceTier::Auto, "auto"),
        (ServiceTier::Default, "default"),
        (ServiceTier::Flex, "flex"),
        (ServiceTier::Priority, "priority"),
        (ServiceTier::Scale, "scale"),
        (ServiceTier::Fast, "fast"),
        (ServiceTier::Ultrafast, "ultrafast"),
    ];
    for (tier, wire) in tiers {
        let options = Options {
            service_tier: Some(tier),
            ..Options::default()
        };
        assert_eq!(body_with(options)["service_tier"], json!(wire));
    }
}

#[test]
fn given_a_cache_mode_when_encoded_then_prompt_cache_options_holds_the_one_member_mode() {
    let implicit = Options {
        cache_mode: Some(PromptCacheMode::Implicit),
        ..Options::default()
    };
    assert_eq!(
        body_with(implicit),
        with_members(json!({ "prompt_cache_options": { "mode": "implicit" } }))
    );
    let explicit = Options {
        cache_mode: Some(PromptCacheMode::Explicit),
        ..Options::default()
    };
    assert_eq!(
        body_with(explicit),
        with_members(json!({ "prompt_cache_options": { "mode": "explicit" } }))
    );
}

#[test]
fn given_an_extra_body_when_encoded_then_its_members_are_added_at_the_top_level() {
    let options = extra(json!({
        "temperature": 0.2,
        "tool_choice": "auto",
        "parallel_tool_calls": false,
        "truncation": "auto",
        "metadata": { "run": "a" }
    }));
    assert_eq!(
        body_with(options),
        with_members(json!({
            "temperature": 0.2,
            "tool_choice": "auto",
            "parallel_tool_calls": false,
            "truncation": "auto",
            "metadata": { "run": "a" }
        }))
    );
}

#[test]
fn given_a_second_extra_body_when_set_then_it_replaces_the_first() {
    let options = extra(json!({ "temperature": 0.2 }))
        .with_extra_body(object(json!({ "top_p": 0.5 })))
        .expect("allowed");
    assert_eq!(body_with(options), with_members(json!({ "top_p": 0.5 })));
}

#[test]
fn given_every_option_set_when_encoded_then_the_whole_body_is_the_golden_json() {
    let options = Options {
        prompt_cache_key: Some("session-42".to_owned()),
        service_tier: Some(ServiceTier::Priority),
        cache_mode: Some(PromptCacheMode::Explicit),
        ..Options::default()
    }
    .with_extra_body(object(json!({ "temperature": 0.2, "top_p": 0.9 })))
    .expect("allowed");
    assert_eq!(
        body_with(options),
        json!({
            "model": "gpt-5.1",
            "stream": true,
            "store": false,
            "max_output_tokens": 4096,
            "input": [{
                "type": "message",
                "role": "user",
                "content": [{ "type": "input_text", "text": "hello" }]
            }],
            "reasoning": { "effort": "none" },
            "prompt_cache_key": "session-42",
            "service_tier": "priority",
            "prompt_cache_options": { "mode": "explicit" },
            "temperature": 0.2,
            "top_p": 0.9
        })
    );
}

#[test]
fn given_the_cache_requirement_required_when_encoded_then_no_cache_option_is_switched_on() {
    let dialect = OpenAiResponses::new(
        &settings(ReasoningLevel::Off, CacheRequirement::Required),
        Options::default(),
    );
    let body = encode_body(&dialect, &hello()).expect("encoded");
    assert_eq!(body, base_body());
}

#[test]
fn given_the_cache_requirement_required_and_a_cache_mode_when_encoded_then_the_mode_is_the_option()
{
    let options = Options {
        cache_mode: Some(PromptCacheMode::Explicit),
        ..Options::default()
    };
    let dialect = OpenAiResponses::new(
        &settings(ReasoningLevel::Off, CacheRequirement::Required),
        options,
    );
    let body = encode_body(&dialect, &hello()).expect("encoded");
    assert_eq!(body["prompt_cache_options"], json!({ "mode": "explicit" }));
    assert!(body.get("prompt_cache_key").is_none());
}
