use br_llm_clients_core::{ClientRequest, OutputMode, ParameterKind};
use br_llm_messages::{ContentBlock, MediaType, TextBlock};
use serde_json::json;

use crate::test_fixtures::*;
use crate::test_thread::*;

pub(crate) fn every_request_shape() -> Vec<ClientRequest> {
    let weather = tool(
        "weather",
        "the weather",
        vec![param("city", "", true, ParameterKind::String)],
    );
    let call = call("t1", "weather", json!({"city": "Paris"}));
    vec![
        request(vec![user("hello")]),
        request_with(
            Some("be brief"),
            vec![user("hello")],
            vec![],
            OutputMode::Plain,
        ),
        request_with(
            None,
            vec![user("hello")],
            vec![weather.clone()],
            OutputMode::Plain,
        ),
        request_with(
            None,
            vec![user("hello")],
            vec![],
            structured(
                Some("an answer"),
                vec![param("v", "", true, ParameterKind::String)],
            ),
        ),
        request(vec![
            user("hello"),
            in_progress(vec![
                calling(vec![
                    thinking("hm", Some(signature("sig"))),
                    redacted(redacted_data("abc")),
                    call_block(&call),
                ]),
                results(vec![result_of(&call, texts(&["sunny"]), false)]),
            ]),
        ]),
        request(vec![
            human_with(vec![
                plain_image(MediaType::Png, "AAAA"),
                plain_text("look"),
            ]),
            assistant_text("seen"),
            user("more"),
        ]),
        request(vec![
            user("hello"),
            one_call_turn(&lookup("t2"), texts(&["bad"]), true),
        ]),
        request(vec![
            user("hello"),
            one_call_turn(
                &lookup("t3"),
                vec![plain_text("see"), plain_image(MediaType::Jpeg, "QUJD")],
                false,
            ),
        ]),
        request(vec![
            user("hello"),
            one_call_turn(&lookup("t4"), Vec::new(), false),
        ]),
        request(vec![user("hello"), sealed_turn()]),
        request_with(
            None,
            vec![user("hello")],
            vec![rich_tool()],
            OutputMode::Plain,
        ),
        request_with(
            None,
            vec![user("hello")],
            vec![],
            structured(
                None,
                vec![param(
                    "point",
                    "",
                    true,
                    ParameterKind::Object {
                        parameters: parameters(vec![param("x", "", true, ParameterKind::Number)]),
                    },
                )],
            ),
        ),
    ]
}

fn lookup(id: &str) -> br_llm_messages::RuntimeToolCall {
    call(id, "lookup", json!({}))
}

fn sealed_text() -> ContentBlock {
    ContentBlock::Text(
        TextBlock::new(block_id(), "sealed")
            .expect("a text")
            .with_replay(material("other/format", "text-replay-secret"))
            .with_item_ref(item_ref("text-item-secret")),
    )
}

fn sealed_call() -> br_llm_messages::RuntimeToolCall {
    call("t5", "lookup", json!({"q": 1}))
        .with_replay(material("other/format", "call-replay-secret"))
        .with_item_ref(item_ref("call-item-secret"))
}

fn sealed_turn() -> br_llm_messages::Turn {
    let call = sealed_call();
    in_progress(vec![
        calling(vec![sealed_text(), call_block(&call)]),
        results(vec![result_of(&call, texts(&["r"]), false)]),
    ])
}

fn rich_tool() -> br_llm_clients_core::ToolSpec {
    tool(
        "find",
        "",
        vec![
            param("q", "the query", true, ParameterKind::String),
            param(
                "mode",
                "",
                false,
                ParameterKind::Enum {
                    values: vec!["a".to_owned(), "b".to_owned()],
                },
            ),
            param(
                "tags",
                "",
                false,
                ParameterKind::Array {
                    items: Box::new(ParameterKind::Integer),
                },
            ),
            param(
                "filter",
                "",
                false,
                ParameterKind::Object {
                    parameters: parameters(vec![param("near", "", true, ParameterKind::Boolean)]),
                },
            ),
        ],
    )
}
