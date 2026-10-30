use br_llm_clients_core::{ClientError, OutputMode, ReasoningLevel};
use br_llm_messages::{ReplayPayload, ResponseEnding, TurnEntry, Usage};
use serde_json::json;

use crate::test_events::{
    arguments_delta, call_added, call_done, completed, created, in_progress, message_added,
    message_done, output_done_part, output_part, part_added, reasoning_added, reasoning_done,
    refusal_delta, refusal_done_part, refusal_part, summary_delta, text_delta, usage_object,
};
use crate::test_history::{plain_text, reply, result_of, results, say};
use crate::test_requests::{dialect, encoded, hello, request, request_with, shape, user_text};
use crate::test_run::{call_stream, calls, kinds, texts, thinkings};

fn text_events(model: &str) -> Vec<serde_json::Value> {
    vec![
        created(model),
        in_progress(),
        message_added(0),
        part_added(0, 0, output_part()),
        text_delta(0, 0, "Hello"),
        text_delta(0, 0, " world"),
        message_done(0, Some("completed"), vec![output_done_part("Hello world")]),
        completed(model, usage_object(37, 11)),
    ]
}

#[tokio::test]
async fn given_a_plain_text_stream_when_run_then_the_reply_holds_the_text_the_usage_the_model_and_the_id()
 {
    let called = call_stream(&hello(), &text_events("gpt-5.1-2026-01-01")).await;
    let (report, result) = called.outcome.into_parts();
    let reply = result.expect("a reply");
    assert_eq!(reply.ending(), &ResponseEnding::finished());
    assert_eq!(texts(&reply), ["Hello world"]);
    assert_eq!(
        reply.usage(),
        Some(
            &Usage::of_input_tokens(37)
                .with_cache_read_input_tokens(0)
                .with_output_tokens(11)
                .with_thinking_tokens(0)
        )
    );
    assert_eq!(
        reply.model().map(|model| model.as_str()),
        Some("gpt-5.1-2026-01-01")
    );
    assert_eq!(
        reply.provider_response_id().map(|id| id.as_str()),
        Some("resp_0b1c2d")
    );
    assert_eq!(
        report.model().map(|model| model.as_str()),
        Some("gpt-5.1-2026-01-01")
    );
}

#[tokio::test]
async fn given_a_call_when_run_then_the_request_is_the_encoded_body_with_the_expected_headers() {
    let called = call_stream(&hello(), &text_events("m")).await;
    assert_eq!(called.requests.len(), 1);
    let received = &called.requests[0];
    assert!(received.request_line.starts_with("POST /v1/responses"));
    assert_eq!(received.header_values("content-type"), ["application/json"]);
    assert_eq!(received.header_values("accept"), ["text/event-stream"]);
    let encoded = encoded(&dialect(ReasoningLevel::Off), &hello()).expect("encoded");
    assert_eq!(received.body, encoded.body());
}

#[tokio::test]
async fn given_a_tool_call_stream_when_run_then_the_reply_uses_a_tool_with_parsed_arguments() {
    let events = vec![
        created("m"),
        call_added(0, "call_Zr1", "get_weather"),
        arguments_delta(0, "{\"city\":"),
        arguments_delta(0, "\"Paris\"}"),
        call_done(
            0,
            "call_Zr1",
            "get_weather",
            "{\"city\":\"Paris\"}",
            Some("completed"),
        ),
        completed("m", usage_object(5, 6)),
    ];
    let reply = call_stream(&hello(), &events).await.reply();
    assert_eq!(reply.ending(), &ResponseEnding::tool_use());
    let found = calls(&reply);
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].external_id().map(|id| id.as_str()),
        Some("call_Zr1")
    );
    assert_eq!(found[0].name().as_str(), "get_weather");
    assert_eq!(
        found[0].arguments().to_json_string().expect("json"),
        "{\"city\":\"Paris\"}"
    );
}

#[tokio::test]
async fn given_text_and_two_calls_when_run_then_the_calls_come_last_in_output_order() {
    let events = vec![
        created("m"),
        call_added(0, "call_a", "first"),
        call_done(0, "call_a", "first", "{}", Some("completed")),
        message_added(1),
        message_done(1, Some("completed"), vec![output_done_part("between")]),
        call_added(2, "call_b", "second"),
        call_done(2, "call_b", "second", "{\"n\":1}", Some("completed")),
        completed("m", usage_object(1, 1)),
    ];
    let reply = call_stream(&hello(), &events).await.reply();
    assert_eq!(kinds(&reply), ["text", "call", "call"]);
    let names: Vec<String> = calls(&reply)
        .iter()
        .map(|call| call.name().as_str().to_owned())
        .collect();
    assert_eq!(names, ["first", "second"]);
}

#[tokio::test]
async fn given_a_call_without_a_call_id_when_run_then_it_has_no_external_id_and_goes_back_with_the_wire_id_of_the_core()
 {
    let mut added = call_added(0, "x", "search");
    added["item"]
        .as_object_mut()
        .expect("item")
        .remove("call_id");
    let mut done = call_done(0, "x", "search", "{}", Some("completed"));
    done["item"]
        .as_object_mut()
        .expect("item")
        .remove("call_id");
    let events = vec![
        created("m"),
        added,
        done,
        completed("m", usage_object(1, 1)),
    ];
    let reply = call_stream(&hello(), &events).await.reply();
    let call = calls(&reply).pop().expect("call");
    assert!(call.external_id().is_none());
    let history = vec![
        user_text("hello"),
        crate::test_history::reply(vec![
            TurnEntry::ModelResponse(reply.clone()),
            results(vec![result_of(&call, vec![plain_text("ok")], false)]),
        ]),
    ];
    let body = crate::test_requests::body_of(&request(history));
    let wire = format!("call_{}", call.id().as_uuid().simple());
    assert_eq!(body["input"][1]["call_id"], json!(wire));
    assert_eq!(body["input"][2]["call_id"], json!(wire));
}

#[tokio::test]
async fn given_a_reasoning_stream_when_run_then_the_thinking_block_carries_the_envelope_and_goes_back_in_the_same_turn()
 {
    let events = vec![
        created("m"),
        reasoning_added(0),
        summary_delta(0, 0, "**Looking** up"),
        summary_delta(0, 1, "then answer"),
        reasoning_done(
            0,
            "rs_68a1",
            Some("gAAAAAB-enc"),
            &["**Looking** up", "then answer"],
        ),
        call_added(1, "call_1", "search"),
        call_done(1, "call_1", "search", "{}", Some("completed")),
        completed("m", usage_object(5, 6)),
    ];
    let first = call_stream(&hello(), &events).await;
    let response = first.reply();
    let thinking = thinkings(&response).pop().expect("thinking");
    assert_eq!(thinking.text(), "**Looking** up\n\nthen answer");
    let material = thinking.replay().expect("replay material");
    assert_eq!(
        material.format().as_str(),
        "br-llm-clients-api-openai-responses/reasoning"
    );
    let ReplayPayload::Text { value, .. } = material.payload() else {
        panic!("a text payload");
    };
    let envelope: serde_json::Value = serde_json::from_str(value).expect("json");
    assert_eq!(
        envelope,
        json!({ "id": "rs_68a1", "encrypted_content": "gAAAAAB-enc" })
    );

    let call = calls(&response).pop().expect("call");
    let history = vec![
        say("hello"),
        reply(vec![
            TurnEntry::ModelResponse(response),
            results(vec![result_of(&call, vec![plain_text("found")], false)]),
        ]),
    ];
    let second = call_stream(&request(history), &text_events("m")).await;
    let body = second.requests[0].body_json();
    assert_eq!(
        body["input"][1],
        json!({ "type": "reasoning", "id": "rs_68a1", "summary": [], "encrypted_content": "gAAAAAB-enc" })
    );
    assert_eq!(body["input"][2]["type"], json!("function_call"));
}

#[tokio::test]
async fn given_a_refusal_in_plain_mode_when_run_then_the_reply_has_the_refusal_ending_and_text() {
    let events = vec![
        created("m"),
        message_added(0),
        part_added(0, 0, refusal_part()),
        refusal_delta(0, 0, "I can't help with that."),
        message_done(
            0,
            Some("completed"),
            vec![refusal_done_part("I can't help with that.")],
        ),
        completed("m", usage_object(2, 3)),
    ];
    let reply = call_stream(&hello(), &events).await.reply();
    assert_eq!(reply.ending(), &ResponseEnding::refusal());
    assert_eq!(texts(&reply), ["I can't help with that."]);
}

#[tokio::test]
async fn given_a_refusal_in_structured_mode_when_run_then_the_content_is_blocked() {
    let events = vec![
        created("m"),
        message_added(0),
        message_done(0, Some("completed"), vec![refusal_done_part("No.")]),
        completed("m", usage_object(2, 3)),
    ];
    let request = request_with(
        None,
        vec![user_text("go")],
        Vec::new(),
        OutputMode::Structured(shape("answer", None, Vec::new())),
    );
    let failure = call_stream(&request, &events).await.failure();
    assert_eq!(failure, ClientError::ContentBlocked);
}

#[tokio::test]
async fn given_a_usage_with_only_the_input_count_when_run_then_the_reply_carries_that_count() {
    let mut events = text_events("m");
    let last = events.len() - 1;
    events[last] = completed("m", json!({ "input_tokens": 9 }));
    let reply = call_stream(&hello(), &events).await.reply();
    assert_eq!(reply.usage(), Some(&Usage::of_input_tokens(9)));
}
