use br_llm_clients_core::{OutputMode, ParameterKind, ReasoningLevel};
use br_llm_messages::{ContentBlock, MediaType};
use serde_json::json;

use crate::test_history::{
    call, envelope_replay, plain_image, plain_text, reply, response, result_of, results, say, text,
    thinking,
};
use crate::test_requests::{dialect, encode_body, encoded, parameter, request, request_with, tool};

#[test]
fn given_the_history_of_the_0_1_golden_when_encoded_then_the_whole_body_is_the_body_of_0_1() {
    let found = call(Some("call_Zr1"), "get_weather", "{\"city\":\"Paris\"}");
    let turns = vec![
        say("What is the weather in Paris?"),
        reply(vec![
            response(vec![
                thinking(
                    "The user wants the weather.",
                    Some(envelope_replay("rs_68a1", "gAAAAABo-encrypted")),
                ),
                text("I will check."),
                ContentBlock::RuntimeToolCall(found.clone()),
            ]),
            results(vec![result_of(
                &found,
                vec![plain_text("18C, cloudy")],
                false,
            )]),
        ]),
    ];
    let request = request_with(
        Some("You are a weather assistant."),
        turns,
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
    let found = call(Some("c"), "search", "{}");
    let request = request(vec![
        say("go"),
        reply(vec![
            response(vec![
                thinking("t", Some(envelope_replay("rs_1", "ENC"))),
                ContentBlock::RuntimeToolCall(found.clone()),
            ]),
            results(vec![result_of(&found, vec![plain_text("r")], false)]),
        ]),
    ]);
    let encoded = encoded(&dialect(ReasoningLevel::Off), &request).expect("encoded");
    let body = String::from_utf8(encoded.body().to_vec()).expect("utf8");
    assert!(body.contains(
        "{\"encrypted_content\":\"ENC\",\"id\":\"rs_1\",\"summary\":[],\"type\":\"reasoning\"}"
    ));
}

#[test]
fn given_a_tool_result_with_an_image_when_encoded_then_the_output_is_the_golden_array() {
    let found = call(Some("call_img"), "render", "{}");
    let request = request(vec![
        say("draw"),
        reply(vec![
            response(vec![ContentBlock::RuntimeToolCall(found.clone())]),
            results(vec![result_of(
                &found,
                vec![
                    plain_text("rendered"),
                    plain_image(MediaType::Png, "iVBORw0K"),
                ],
                true,
            )]),
        ]),
    ]);
    let body = encode_body(&dialect(ReasoningLevel::Off), &request).expect("encoded");
    assert_eq!(
        body["input"],
        json!([
            {
                "type": "message",
                "role": "user",
                "content": [{ "type": "input_text", "text": "draw" }]
            },
            { "type": "function_call", "call_id": "call_img", "name": "render", "arguments": "{}" },
            {
                "type": "function_call_output",
                "call_id": "call_img",
                "output": [
                    { "type": "input_text", "text": "Error: " },
                    { "type": "input_text", "text": "rendered" },
                    { "type": "input_image", "image_url": "data:image/png;base64,iVBORw0K" }
                ]
            }
        ])
    );
}
