use br_llm_clients_core::{OutputMode, ParameterKind, ReasoningLevel};
use br_llm_messages::AssistantBlock;
use serde_json::json;

use crate::test_requests::{
    assistant, call_block, dialect, encode_body, envelope_text, parameter, request_with,
    result_block, text, thinking, tool, user, user_text,
};

#[test]
fn given_an_agent_turn_with_reasoning_when_encoded_then_the_whole_body_is_the_golden_json() {
    let signature = envelope_text("rs_68a1", "gAAAAABo-encrypted");
    let messages = vec![
        user_text("What is the weather in Paris?"),
        assistant(vec![
            thinking("The user wants the weather.", Some(&signature)),
            AssistantBlock::Text {
                text: text("I will check."),
            },
            call_block("call_Zr1", "get_weather", json!({ "city": "Paris" })),
        ]),
        user(vec![result_block(
            "call_Zr1",
            "get_weather",
            &["18C, cloudy"],
            false,
        )]),
    ];
    let request = request_with(
        Some("You are a weather assistant."),
        messages,
        vec![tool(
            "get_weather",
            "Current weather of a city",
            vec![parameter(
                "city",
                "The city name",
                true,
                ParameterKind::String,
            )],
        )],
        OutputMode::Plain,
    );
    let body = encode_body(&dialect(ReasoningLevel::High), &request).expect("encoded");
    assert_eq!(
        body,
        json!({
            "model": "gpt-5.1",
            "stream": true,
            "store": false,
            "instructions": "You are a weather assistant.",
            "max_output_tokens": 4096,
            "reasoning": { "effort": "high", "summary": "auto" },
            "include": ["reasoning.encrypted_content"],
            "tools": [{
                "type": "function",
                "name": "get_weather",
                "description": "Current weather of a city",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "city": { "type": "string", "description": "The city name" }
                    },
                    "required": ["city"],
                    "additionalProperties": false
                },
                "strict": true
            }],
            "input": [
                {
                    "type": "message",
                    "role": "user",
                    "content": [{ "type": "input_text", "text": "What is the weather in Paris?" }]
                },
                {
                    "type": "reasoning",
                    "id": "rs_68a1",
                    "summary": [],
                    "encrypted_content": "gAAAAABo-encrypted"
                },
                { "type": "message", "role": "assistant", "content": "I will check." },
                {
                    "type": "function_call",
                    "call_id": "call_Zr1",
                    "name": "get_weather",
                    "arguments": "{\"city\":\"Paris\"}"
                },
                {
                    "type": "function_call_output",
                    "call_id": "call_Zr1",
                    "output": "18C, cloudy"
                }
            ]
        })
    );
}

#[test]
fn given_the_replayed_reasoning_item_when_serialized_then_the_text_is_the_golden_text() {
    let signature = envelope_text("rs_1", "ENC");
    let messages = vec![
        user_text("go"),
        assistant(vec![
            thinking("t", Some(&signature)),
            call_block("c", "search", json!({})),
        ]),
        user(vec![result_block("c", "search", &["r"], false)]),
    ];
    let request = request_with(None, messages, Vec::new(), OutputMode::Plain);
    let encoded =
        br_llm_clients_common_call::Dialect::encode(&dialect(ReasoningLevel::Off), &request)
            .expect("encoded");
    let body = String::from_utf8(encoded.body().to_vec()).expect("utf8");
    assert!(body.contains(
        "{\"encrypted_content\":\"ENC\",\"id\":\"rs_1\",\"summary\":[],\"type\":\"reasoning\"}"
    ));
}
