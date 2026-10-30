use br_llm_clients_core::{
    CacheRequirement, ClientError, ClientSettings, MaxOutputTokens, ParameterKind,
};
use serde_json::{Value, json};

use crate::test_support::{
    dialect, dialect_for, encode, encoded_body, hello, options, parameter, request_of,
    request_with, shape, tool,
};
use crate::test_thread::user;
use crate::{
    ChatCompletionsDialect, ChatCompletionsOptions, ReasoningEffort, ReasoningLevel, ReasoningRule,
    ReasoningTable,
};

fn members(body: &Value) -> Vec<String> {
    let mut names: Vec<String> = body.as_object().expect("object").keys().cloned().collect();
    names.sort();
    names
}

#[test]
fn given_a_plain_request_when_encoded_then_the_body_is_exactly_the_documented_members() {
    let body = encoded_body(&dialect(), &hello());
    assert_eq!(
        body,
        json!({
            "model": "gpt-test",
            "messages": [{"role": "user", "content": "hello"}],
            "stream": true,
            "stream_options": {"include_usage": true},
            "max_completion_tokens": 1024,
            "reasoning_effort": "none"
        })
    );
}

#[test]
fn given_a_system_prompt_when_encoded_then_it_is_the_first_message_with_a_plain_string() {
    let request = request_with(
        Some("be brief"),
        vec![user("hello")],
        Vec::new(),
        br_llm_clients_core::OutputMode::Plain,
    );
    let body = encoded_body(&dialect(), &request);
    assert_eq!(
        body["messages"],
        json!([
            {"role": "system", "content": "be brief"},
            {"role": "user", "content": "hello"}
        ])
    );
}

#[test]
fn given_no_system_prompt_when_encoded_then_no_system_message_is_written() {
    let body = encoded_body(&dialect(), &hello());
    let roles: Vec<&str> = body["messages"]
        .as_array()
        .expect("messages")
        .iter()
        .filter_map(|message| message["role"].as_str())
        .collect();
    assert_eq!(roles, ["user"]);
}

#[test]
fn given_tools_a_shape_and_an_extra_body_when_encoded_then_no_member_is_written_beyond_the_listed_ones()
 {
    let extra = json!({"temperature": 0.2, "top_p": 0.9});
    let configured = options().with_extra_body(extra.as_object().cloned().expect("object"));
    let dialect = dialect_for(ReasoningLevel::High, configured);
    let request = request_with(
        Some("system"),
        vec![user("hello")],
        vec![tool("lookup", "", Vec::new())],
        shape("answer", None, Vec::new()),
    );
    assert_eq!(
        members(&encoded_body(&dialect, &request)),
        [
            "max_completion_tokens",
            "messages",
            "model",
            "reasoning_effort",
            "response_format",
            "stream",
            "stream_options",
            "temperature",
            "tools",
            "top_p"
        ]
    );
}

#[test]
fn given_no_tools_and_plain_output_when_encoded_then_neither_member_is_written() {
    let body = encoded_body(&dialect(), &hello());
    assert!(body.get("tools").is_none());
    assert!(body.get("response_format").is_none());
    assert!(body.get("tool_choice").is_none());
    assert!(body.get("parallel_tool_calls").is_none());
}

#[test]
fn given_an_extra_body_when_encoded_then_its_members_sit_at_the_top_level_with_their_values() {
    let extra = json!({"temperature": 0.2, "metadata": {"a": [1, 2]}});
    let configured = options().with_extra_body(extra.as_object().cloned().expect("object"));
    let body = encoded_body(&dialect_for(ReasoningLevel::Off, configured), &hello());
    assert_eq!(body["temperature"], json!(0.2));
    assert_eq!(body["metadata"], json!({"a": [1, 2]}));
}

#[test]
fn given_the_settings_when_encoded_then_the_model_and_the_token_limit_come_from_them() {
    let settings = ClientSettings::new(
        crate::ModelName::new("o3-mini").expect("model"),
        ReasoningLevel::Off,
        CacheRequirement::NotRequired,
        MaxOutputTokens::new(4_000_000_000).expect("tokens"),
    );
    let dialect = ChatCompletionsDialect::new(settings, options()).expect("dialect");
    let body = encoded_body(&dialect, &hello());
    assert_eq!(body["model"], json!("o3-mini"));
    assert_eq!(body["max_completion_tokens"], json!(4_000_000_000u64));
    assert!(body["max_completion_tokens"].is_u64());
}

#[test]
fn given_a_request_when_encoded_then_the_only_header_is_the_json_content_type() {
    let encoded = encode(&dialect(), &hello()).expect("encoded");
    assert_eq!(
        encoded.headers(),
        [("Content-Type".to_owned(), "application/json".to_owned())]
    );
}

#[test]
fn given_either_cache_requirement_when_encoded_then_nothing_is_written_and_nothing_is_refused() {
    let mut bodies = Vec::new();
    for cache in [CacheRequirement::Required, CacheRequirement::NotRequired] {
        let settings = ClientSettings::new(
            crate::ModelName::new("gpt-test").expect("model"),
            ReasoningLevel::Off,
            cache,
            MaxOutputTokens::new(1024).expect("tokens"),
        );
        let dialect = ChatCompletionsDialect::new(settings, options()).expect("dialect");
        let encoded = encode(&dialect, &hello()).expect("not refused");
        bodies.push(encoded.body().to_vec());
    }
    assert_eq!(bodies.first(), bodies.get(1));
    let body: Value = serde_json::from_slice(bodies.first().expect("body")).expect("json");
    assert!(!body.to_string().contains("cache"));
}

#[test]
fn given_the_same_request_when_encoded_twice_then_the_bytes_are_equal() {
    let request = request_with(
        Some("s"),
        vec![user("hello")],
        vec![tool(
            "lookup",
            "finds",
            vec![
                parameter("b", "", true, ParameterKind::String),
                parameter("a", "", false, ParameterKind::Integer),
            ],
        )],
        shape("answer", Some("d"), Vec::new()),
    );
    let dialect = dialect();
    assert_eq!(encode(&dialect, &request), encode(&dialect, &request));
}

#[test]
fn given_the_effort_table_when_each_level_is_encoded_then_the_documented_effort_is_sent() {
    let expected = [
        (ReasoningLevel::Off, "none"),
        (ReasoningLevel::Low, "low"),
        (ReasoningLevel::Medium, "medium"),
        (ReasoningLevel::High, "high"),
        (ReasoningLevel::Extra, "xhigh"),
        (ReasoningLevel::Max, "max"),
    ];
    for (level, effort) in expected {
        let body = encoded_body(&dialect_for(level, options()), &hello());
        assert_eq!(body["reasoning_effort"], json!(effort), "{level:?}");
    }
}

#[test]
fn given_the_table_without_none_when_each_level_is_encoded_then_off_is_refused_and_the_top_is_high()
{
    let table = || ChatCompletionsOptions::new(ReasoningTable::openai_effort_without_none());
    let refused = encode(&dialect_for(ReasoningLevel::Off, table()), &hello());
    assert!(matches!(
        refused,
        Err(ClientError::ReasoningNotSupported {
            level: ReasoningLevel::Off,
            ..
        })
    ));
    let expected = [
        (ReasoningLevel::Low, "low"),
        (ReasoningLevel::Medium, "medium"),
        (ReasoningLevel::High, "high"),
        (ReasoningLevel::Extra, "high"),
        (ReasoningLevel::Max, "high"),
    ];
    for (level, effort) in expected {
        let body = encoded_body(&dialect_for(level, table()), &hello());
        assert_eq!(body["reasoning_effort"], json!(effort), "{level:?}");
    }
}

#[test]
fn given_an_omit_rule_for_off_when_encoded_then_no_effort_member_is_written() {
    let table = ReasoningTable::new(
        ReasoningRule::Omit,
        ReasoningRule::Send(ReasoningEffort::Low),
        ReasoningRule::Send(ReasoningEffort::Medium),
        ReasoningRule::Send(ReasoningEffort::High),
        ReasoningRule::Send(ReasoningEffort::High),
        ReasoningRule::Send(ReasoningEffort::High),
    );
    let body = encoded_body(
        &dialect_for(ReasoningLevel::Off, ChatCompletionsOptions::new(table)),
        &hello(),
    );
    assert!(body.get("reasoning_effort").is_none());
}

#[test]
fn given_a_minimal_effort_rule_when_encoded_then_minimal_is_sent_as_text() {
    let table = ReasoningTable::new(
        ReasoningRule::Refuse,
        ReasoningRule::Send(ReasoningEffort::Minimal),
        ReasoningRule::Refuse,
        ReasoningRule::Refuse,
        ReasoningRule::Refuse,
        ReasoningRule::Refuse,
    );
    let body = encoded_body(
        &dialect_for(ReasoningLevel::Low, ChatCompletionsOptions::new(table)),
        &hello(),
    );
    assert_eq!(body["reasoning_effort"], json!("minimal"));
}

#[test]
fn given_two_requests_when_encoded_in_turn_then_the_dialect_holds_no_per_call_state() {
    let dialect = dialect();
    let first = encode(&dialect, &hello()).expect("first");
    let other = request_of(vec![user("different")]);
    let _ = encode(&dialect, &other).expect("other");
    assert_eq!(encode(&dialect, &hello()).expect("again"), first);
}

#[test]
fn given_different_token_limits_when_encoded_then_the_reasoning_effort_does_not_change() {
    for limit in [1, 1024, u32::MAX] {
        let settings = ClientSettings::new(
            crate::ModelName::new("gpt-test").expect("model"),
            ReasoningLevel::High,
            CacheRequirement::NotRequired,
            MaxOutputTokens::new(limit).expect("tokens"),
        );
        let dialect = ChatCompletionsDialect::new(settings, options()).expect("dialect");
        let body = encoded_body(&dialect, &hello());
        assert_eq!(body["reasoning_effort"], json!("high"));
        assert_eq!(body["max_completion_tokens"], json!(limit));
    }
}
