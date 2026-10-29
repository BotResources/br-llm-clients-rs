use serde_json::{Value, json};

use br_llm_clients_core::{CacheRequirement, ReasoningLevel};

use crate::test_fixtures::*;
use crate::test_shapes::every_request_shape;
use crate::{AnthropicMessages, MessagesOptions};

fn one_call(id: &str, result: Value) -> Value {
    json!([
        {"role": "user", "content": [{"type": "text", "text": "hello"}]},
        {"role": "assistant", "content": [{"type": "tool_use", "id": id, "name": "lookup", "input": {}}]},
        {"role": "user", "content": [result]}
    ])
}

fn expected_bodies() -> Vec<Value> {
    let hello = json!([{"role": "user", "content": [{"type": "text", "text": "hello"}]}]);
    let base = |messages: Value| {
        json!({
            "model": MODEL,
            "max_tokens": 1024,
            "stream": true,
            "messages": messages,
            "thinking": {"type": "disabled"}
        })
    };
    let with = |mut body: Value, member: &str, value: Value| {
        body[member] = value;
        body
    };
    vec![
        base(hello.clone()),
        with(base(hello.clone()), "system", json!("be brief")),
        with(
            base(hello.clone()),
            "tools",
            json!([{
                "name": "weather",
                "description": "the weather",
                "input_schema": {
                    "type": "object",
                    "properties": {"city": {"type": "string"}},
                    "required": ["city"]
                }
            }]),
        ),
        with(
            base(hello.clone()),
            "output_config",
            json!({"format": {"type": "json_schema", "schema": {
                "type": "object",
                "description": "an answer",
                "properties": {"v": {"type": "string"}},
                "required": ["v"],
                "additionalProperties": false
            }}}),
        ),
        base(json!([
            {"role": "user", "content": [{"type": "text", "text": "hello"}]},
            {"role": "assistant", "content": [
                {"type": "thinking", "thinking": "hm", "signature": "sig"},
                {"type": "redacted_thinking", "data": "abc"},
                {"type": "tool_use", "id": "t1", "name": "weather", "input": {"city": "Paris"}}
            ]},
            {"role": "user", "content": [{
                "type": "tool_result",
                "tool_use_id": "t1",
                "content": [{"type": "text", "text": "sunny"}]
            }]}
        ])),
        base(json!([
            {"role": "user", "content": [
                {"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": "AAAA"}},
                {"type": "text", "text": "look"}
            ]},
            {"role": "assistant", "content": [{"type": "text", "text": "seen"}]},
            {"role": "user", "content": [{"type": "text", "text": "more"}]}
        ])),
        base(one_call(
            "t2",
            json!({"type": "tool_result", "tool_use_id": "t2",
            "content": [{"type": "text", "text": "bad"}], "is_error": true}),
        )),
        base(one_call(
            "t3",
            json!({"type": "tool_result", "tool_use_id": "t3", "content": [
                {"type": "text", "text": "see"},
                {"type": "image", "source": {"type": "base64", "media_type": "image/jpeg", "data": "QUJD"}}
            ]}),
        )),
        base(one_call(
            "t4",
            json!({"type": "tool_result", "tool_use_id": "t4"}),
        )),
        base(json!([
            {"role": "user", "content": [{"type": "text", "text": "hello"}]},
            {"role": "assistant", "content": [
                {"type": "text", "text": "sealed"},
                {"type": "tool_use", "id": "t5", "name": "lookup", "input": {"q": 1}}
            ]},
            {"role": "user", "content": [{
                "type": "tool_result",
                "tool_use_id": "t5",
                "content": [{"type": "text", "text": "r"}]
            }]}
        ])),
        with(
            base(hello.clone()),
            "tools",
            json!([{
                "name": "find",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "q": {"type": "string", "description": "the query"},
                        "mode": {"type": "string", "enum": ["a", "b"]},
                        "tags": {"type": "array", "items": {"type": "integer"}},
                        "filter": {
                            "type": "object",
                            "properties": {"near": {"type": "boolean"}},
                            "required": ["near"]
                        }
                    },
                    "required": ["q"]
                }
            }]),
        ),
        with(
            base(hello.clone()),
            "output_config",
            json!({"format": {"type": "json_schema", "schema": {
                "type": "object",
                "properties": {"point": {
                    "type": "object",
                    "properties": {"x": {"type": "number"}},
                    "required": ["x"],
                    "additionalProperties": false
                }},
                "required": ["point"],
                "additionalProperties": false
            }}}),
        ),
    ]
}

#[test]
fn given_the_default_options_when_encoded_then_with_options_gives_the_bytes_and_headers_of_new() {
    let expected = expected_bodies();
    let shapes = every_request_shape();
    assert_eq!(expected.len(), shapes.len());
    for (shape, expected) in shapes.iter().zip(expected) {
        let by_new = encode(&dialect(), shape).expect("new");
        let by_options = encode(&dialect_options(MessagesOptions::new()), shape).expect("options");
        assert_eq!(by_new.body(), by_options.body());
        assert_eq!(
            by_options.headers(),
            &[
                ("content-type".to_owned(), "application/json".to_owned()),
                ("anthropic-version".to_owned(), "2023-06-01".to_owned()),
            ]
        );
        assert_eq!(by_new.headers(), by_options.headers());
        let body: Value = serde_json::from_slice(by_options.body()).expect("json");
        assert_eq!(body, expected);
    }
}

#[test]
fn given_the_default_options_when_compared_then_new_and_default_are_equal() {
    assert_eq!(MessagesOptions::new(), MessagesOptions::default());
}

#[test]
fn given_each_reasoning_level_and_cache_when_encoded_with_default_options_then_the_body_is_the_one_of_new()
 {
    let levels = [
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
    let hello = request(vec![user("hello")]);
    for (level, thinking, effort) in levels {
        for cache in [CacheRequirement::NotRequired, CacheRequirement::Required] {
            let mut expected = json!({
                "model": MODEL,
                "max_tokens": 1024,
                "stream": true,
                "messages": [{"role": "user", "content": [{"type": "text", "text": "hello"}]}],
                "thinking": thinking.clone()
            });
            if let Some(effort) = effort {
                expected["output_config"] = json!({"effort": effort});
            }
            if cache == CacheRequirement::Required {
                expected["cache_control"] = json!({"type": "ephemeral"});
            }
            let by_new = encode(&dialect_with(level, cache), &hello).expect("new");
            let by_options = encode(
                &AnthropicMessages::with_options(
                    &settings(level, cache, 1024),
                    MessagesOptions::new(),
                ),
                &hello,
            )
            .expect("options");
            assert_eq!(by_new, by_options, "{level:?} {cache:?}");
            let body: Value = serde_json::from_slice(by_options.body()).expect("json");
            assert_eq!(body, expected, "{level:?} {cache:?}");
        }
    }
}
