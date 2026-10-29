use br_llm_clients_core::{ClientRequest, OutputMode, ParameterKind};
use br_llm_messages::MediaType;
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
    ]
}
