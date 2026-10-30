use br_llm_clients_core::{ClientRequest, OutputMode, ParameterKind, ReasoningLevel, ToolSpec};
use br_llm_messages::{MediaType, Turn};
use serde_json::json;

use crate::test_support::{parameter, request_of, request_with, shape, tool};
use crate::test_thread::{
    answer, call, call_block, calling, earlier, human, in_progress, plain_image, plain_text,
    redacted, result_of, results, signed, structured, text, texts, thinking, user,
};
use crate::{ChatCompletionsOptions, ReasoningTable};

pub(crate) struct GoldenCase {
    pub(crate) name: &'static str,
    pub(crate) level: ReasoningLevel,
    pub(crate) options: ChatCompletionsOptions,
    pub(crate) request: ClientRequest,
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

fn tool_loop() -> Vec<Turn> {
    let weather = call("call_1", "weather", json!({"city": "Paris"}));
    let time = call("call_2", "time", json!({}));
    let rome = call("call_3", "weather", json!({"city": "Rome"}));
    vec![
        user("weather in Paris?"),
        in_progress(vec![
            calling(vec![
                thinking("pondering"),
                signed("pondering"),
                text("let me check"),
                call_block(&weather),
                call_block(&time),
            ]),
            results(vec![
                result_of(&weather, texts(&["sunny"]), false),
                result_of(&time, Vec::new(), true),
            ]),
            calling(vec![thinking("pondering"), redacted(), call_block(&rome)]),
            results(vec![result_of(&rome, texts(&["a", "b"]), false)]),
        ]),
    ]
}

fn tools() -> Vec<ToolSpec> {
    vec![
        tool(
            "weather",
            "gives the weather",
            vec![
                parameter("city", "the city", true, ParameterKind::String),
                parameter("days", "", false, ParameterKind::Integer),
            ],
        ),
        tool("time", "", vec![]),
    ]
}

pub(crate) fn golden_cases() -> Vec<GoldenCase> {
    vec![
        GoldenCase {
            name: "hello",
            level: ReasoningLevel::Off,
            options: plain_options(),
            request: request_of(vec![user("hello")]),
        },
        GoldenCase {
            name: "system_and_images",
            level: ReasoningLevel::Medium,
            options: plain_options(),
            request: request_with(
                Some("be brief"),
                vec![human(vec![
                    plain_text("look"),
                    plain_image(MediaType::Png, "QUJD"),
                    plain_text("and this"),
                ])],
                Vec::new(),
                OutputMode::Plain,
            ),
        },
        GoldenCase {
            name: "tool_loop_with_tools",
            level: ReasoningLevel::High,
            options: plain_options(),
            request: request_with(Some("sys"), tool_loop(), tools(), OutputMode::Plain),
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
                    user("go"),
                    earlier(vec![answer(vec![
                        text("a"),
                        text("b"),
                        structured(json!({"k": [1, 2]})),
                    ])]),
                    user("again"),
                ],
                tools(),
                shape(
                    "answer",
                    Some("the answer"),
                    vec![parameter("value", "the value", true, ParameterKind::String)],
                ),
            ),
        },
        GoldenCase {
            name: "two_texts",
            level: ReasoningLevel::Max,
            options: plain_options(),
            request: request_of(vec![human(texts(&["note one", "note two"]))]),
        },
    ]
}
