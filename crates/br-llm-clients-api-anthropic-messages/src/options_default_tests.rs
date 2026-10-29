use serde_json::{Value, json};

use crate::MessagesOptions;
use crate::test_fixtures::*;
use crate::test_shapes::every_request_shape;

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
