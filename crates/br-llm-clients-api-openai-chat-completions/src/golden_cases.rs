use br_llm_clients_core::{OutputMode, ParameterKind, ReasoningLevel};
use br_llm_messages::{
    AssistantBlock, ImageMime, RedactedThinking, Signature, Thinking, WireMessage, WireUserBlock,
};
use serde_json::json;

use crate::ChatCompletionsOptions;
use crate::ReasoningTable;
use crate::test_support::{
    assistant, assistant_call, assistant_text, image, parameter, request_of, request_with, shape,
    text, tool, tool_result, user, user_text,
};

pub(crate) struct GoldenCase {
    pub(crate) name: &'static str,
    pub(crate) level: ReasoningLevel,
    pub(crate) options: ChatCompletionsOptions,
    pub(crate) request: br_llm_clients_core::ClientRequest,
}

fn plain_options() -> ChatCompletionsOptions {
    ChatCompletionsOptions::new(ReasoningTable::openai_effort())
}

fn extra_options() -> ChatCompletionsOptions {
    let mut extra = serde_json::Map::new();
    extra.insert("temperature".to_owned(), json!(0.2));
    extra.insert("max_tokens".to_owned(), json!(7));
    plain_options()
        .with_extra_body(extra)
        .with_reasoning_field("reasoning_content")
        .with_cache_read_field("cached")
}

fn thinking(signature: Option<&str>) -> AssistantBlock {
    AssistantBlock::Thinking(Thinking {
        text: "pondering".to_owned(),
        signature: signature.map(|value| Signature::new(value).expect("signature")),
    })
}

fn tool_loop() -> Vec<WireMessage> {
    vec![
        user_text("weather in Paris?"),
        assistant(vec![
            thinking(None),
            thinking(Some("sig==")),
            assistant_text("let me check"),
            assistant_call("call_1", "weather", json!({"city": "Paris"})),
            assistant_call("call_2", "time", json!({})),
        ]),
        user(vec![
            tool_result("call_1", "weather", &["sunny"], false),
            tool_result("call_2", "time", &[], true),
            tool_result("call_3", "other", &["a", "b"], false),
        ]),
        assistant(vec![
            thinking(None),
            AssistantBlock::RedactedThinking(RedactedThinking::new("opaque").expect("redacted")),
        ]),
        assistant(vec![assistant_call(
            "call_4",
            "weather",
            json!({"city": "Rome"}),
        )]),
        WireMessage::Relay {
            content: vec![text("note one"), text("note two")],
        },
    ]
}

pub(crate) fn golden_cases() -> Vec<GoldenCase> {
    let tools = vec![
        tool(
            "weather",
            "gives the weather",
            vec![
                parameter("city", "the city", true, ParameterKind::String),
                parameter("days", "", false, ParameterKind::Integer),
            ],
        ),
        tool("time", "", vec![]),
    ];
    vec![
        GoldenCase {
            name: "hello",
            level: ReasoningLevel::Off,
            options: plain_options(),
            request: request_of(vec![user_text("hello")]),
        },
        GoldenCase {
            name: "system_and_images",
            level: ReasoningLevel::Medium,
            options: plain_options(),
            request: request_with(
                Some("be brief"),
                vec![user(vec![
                    WireUserBlock::Text { text: text("look") },
                    image(ImageMime::Png, "QUJD"),
                    WireUserBlock::Text {
                        text: text("and this"),
                    },
                ])],
                Vec::new(),
                OutputMode::Plain,
            ),
        },
        GoldenCase {
            name: "tool_loop_with_tools",
            level: ReasoningLevel::High,
            options: plain_options(),
            request: request_with(Some("sys"), tool_loop(), tools.clone(), OutputMode::Plain),
        },
        GoldenCase {
            name: "tool_loop_extra_body",
            level: ReasoningLevel::Off,
            options: extra_options(),
            request: request_with(None, tool_loop(), Vec::new(), OutputMode::Plain),
        },
        GoldenCase {
            name: "structured_with_tools",
            level: ReasoningLevel::Low,
            options: plain_options(),
            request: request_with(
                None,
                vec![
                    user_text("go"),
                    assistant(vec![
                        assistant_text("a"),
                        assistant_text("b"),
                        AssistantBlock::Structured {
                            value: json!({"k": [1, 2]}),
                        },
                    ]),
                    user_text("again"),
                ],
                tools,
                shape(
                    "answer",
                    Some("the answer"),
                    vec![parameter("value", "the value", true, ParameterKind::String)],
                ),
            ),
        },
    ]
}
