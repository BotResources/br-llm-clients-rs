use br_llm_clients_core::{CacheRequirement, MaxOutputTokens, ReasoningLevel};
use serde_json::json;

use crate::AnthropicMessages;
use crate::test_fixtures::*;

#[test]
fn given_each_level_when_encoded_then_thinking_and_effort_follow_the_table() {
    let table = [
        (ReasoningLevel::Off, json!({"type": "disabled"}), None),
        (
            ReasoningLevel::Low,
            json!({"type": "adaptive"}),
            Some("low"),
        ),
        (
            ReasoningLevel::Medium,
            json!({"type": "adaptive"}),
            Some("medium"),
        ),
        (
            ReasoningLevel::High,
            json!({"type": "adaptive"}),
            Some("high"),
        ),
        (
            ReasoningLevel::Extra,
            json!({"type": "adaptive"}),
            Some("xhigh"),
        ),
        (
            ReasoningLevel::Max,
            json!({"type": "adaptive"}),
            Some("max"),
        ),
    ];
    for (level, thinking, effort) in table {
        let body = body_of(
            &dialect_with(level, CacheRequirement::NotRequired),
            &request(vec![user("hello")]),
        );
        assert_eq!(body["thinking"], thinking, "{level:?}");
        match effort {
            Some(effort) => assert_eq!(body["output_config"], json!({"effort": effort})),
            None => assert!(body.get("output_config").is_none(), "{level:?}"),
        }
    }
}

#[test]
fn given_the_table_when_walked_upward_then_only_off_is_disabled() {
    let levels = [
        ReasoningLevel::Off,
        ReasoningLevel::Low,
        ReasoningLevel::Medium,
        ReasoningLevel::High,
        ReasoningLevel::Extra,
        ReasoningLevel::Max,
    ];
    let efforts = ["low", "medium", "high", "xhigh", "max"];
    let sent: Vec<_> = levels
        .iter()
        .map(|level| {
            body_of(
                &dialect_with(*level, CacheRequirement::NotRequired),
                &request(vec![user("hello")]),
            )
        })
        .collect();
    for (position, body) in sent.iter().enumerate() {
        let disabled = body["thinking"]["type"] == json!("disabled");
        assert_eq!(disabled, position == 0);
    }
    let sent_efforts: Vec<_> = sent
        .iter()
        .filter_map(|body| body["output_config"]["effort"].as_str())
        .collect();
    assert_eq!(sent_efforts, efforts);
}

#[test]
fn given_any_max_tokens_when_encoded_then_the_reasoning_fields_do_not_change() {
    for max_tokens in [1, 1024, 64_000, u32::MAX] {
        let dialect = AnthropicMessages::new(&settings(
            ReasoningLevel::High,
            CacheRequirement::NotRequired,
            max_tokens,
        ));
        let body = body_of(&dialect, &request(vec![user("hello")]));
        assert_eq!(body["thinking"], json!({"type": "adaptive"}));
        assert_eq!(body["output_config"], json!({"effort": "high"}));
        assert_eq!(body["max_tokens"], json!(max_tokens));
    }
}

#[test]
fn given_a_zero_max_tokens_when_built_then_core_refuses_it_before_the_dialect() {
    assert!(MaxOutputTokens::new(0).is_err());
}

#[test]
fn given_a_required_cache_when_encoded_then_the_cache_control_is_the_ephemeral_default() {
    let body = body_of(
        &dialect_with(ReasoningLevel::Off, CacheRequirement::Required),
        &request(vec![user("hello")]),
    );
    assert_eq!(body["cache_control"], json!({"type": "ephemeral"}));
}

#[test]
fn given_a_required_cache_when_encoded_then_no_block_tool_or_system_carries_a_marker() {
    let weather = tool("weather", "", vec![]);
    let request = request_with(
        Some("sys"),
        vec![user("hello")],
        vec![weather],
        br_llm_clients_core::OutputMode::Plain,
    );
    let body = body_of(
        &dialect_with(ReasoningLevel::Off, CacheRequirement::Required),
        &request,
    );
    let mut without_top_level = body.clone();
    without_top_level
        .as_object_mut()
        .expect("object")
        .remove("cache_control");
    assert!(!without_top_level.to_string().contains("cache_control"));
}

#[test]
fn given_a_not_required_cache_when_encoded_then_no_cache_field_is_sent() {
    assert!(simple_body().get("cache_control").is_none());
}
