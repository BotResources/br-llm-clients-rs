use br_llm_clients_core::{ClientRequest, OutputMode, ParameterKind};
use br_llm_messages::ImageMime;
use serde_json::json;

use crate::test_fixtures::*;

pub(crate) fn every_request_shape() -> Vec<ClientRequest> {
    let weather = tool(
        "weather",
        "the weather",
        vec![param("city", "", true, ParameterKind::String)],
    );
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
            assistant(vec![
                thinking("hm", Some("sig")),
                redacted("abc"),
                tool_call("t1", "weather", json!({"city": "Paris"})),
            ]),
            user_blocks(vec![tool_result("t1", "weather", &["sunny"], false)]),
        ]),
        request(vec![
            user_blocks(vec![image(ImageMime::Png, "AAAA"), user_text_block("look")]),
            assistant_text("seen"),
            relay(&["more"]),
        ]),
    ]
}
