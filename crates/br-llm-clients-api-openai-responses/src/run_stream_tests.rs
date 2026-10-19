use br_llm_clients_common_call::Dialect;
use br_llm_clients_core::{ClientError, OutputMode, ParameterKind, ReasoningLevel};
use br_llm_messages::{AssistantBlock, StopReason, Usage, WireMessage};
use serde_json::json;

use crate::test_events::{
    arguments_delta, call_added, call_done, completed, created, in_progress, incomplete,
    message_added, message_done, output_done_part, output_part, part_added, reasoning_added,
    reasoning_done, refusal_delta, refusal_done_part, refusal_part, summary_delta, text_delta,
    usage_object,
};
use crate::test_requests::{
    assistant, dialect, hello, parameter, request, request_with, result_block, shape, text, tool,
    user, user_text,
};
use crate::test_run::{call_stream, call_with_wire, ok_wire, sse};

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
async fn given_a_plain_text_stream_when_run_then_the_step_holds_the_text_the_usage_and_the_model() {
    let called = call_stream(&hello(), &text_events("gpt-5.1-2026-01-01")).await;
    let (report, result) = called.outcome.into_parts();
    let step = result.expect("a step");
    assert_eq!(step.stop_reason(), &StopReason::EndTurn);
    let texts: Vec<&str> = step.text().map(|text| text.as_str()).collect();
    assert_eq!(texts, ["Hello world"]);
    assert_eq!(
        step.usage(),
        Some(&Usage {
            input_tokens: 37,
            output_tokens: 11,
            cache_read_tokens: Some(0),
            cache_write_tokens: None,
            thinking_tokens: Some(0),
        })
    );
    assert_eq!(
        step.model().map(|model| model.as_str()),
        Some("gpt-5.1-2026-01-01")
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
    let encoded = dialect(ReasoningLevel::Off)
        .encode(&hello())
        .expect("encoded");
    assert_eq!(received.body, encoded.body());
}

#[tokio::test]
async fn given_a_tool_call_stream_when_run_then_the_step_awaits_tool_results_with_parsed_arguments()
{
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
    let called = call_stream(&hello(), &events).await;
    let step = called.outcome.into_parts().1.expect("a step");
    assert_eq!(step.stop_reason(), &StopReason::AwaitingToolResults);
    let calls: Vec<_> = step.tool_calls().collect();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].id.as_str(), "call_Zr1");
    assert_eq!(calls[0].name.as_str(), "get_weather");
    assert_eq!(calls[0].arguments, json!({ "city": "Paris" }));
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
    let step = call_stream(&hello(), &events)
        .await
        .outcome
        .into_parts()
        .1
        .expect("a step");
    let kinds: Vec<&str> = step
        .content()
        .iter()
        .map(|block| match block {
            AssistantBlock::Text { .. } => "text",
            AssistantBlock::ToolCall(_) => "call",
            _ => "other",
        })
        .collect();
    assert_eq!(kinds, ["text", "call", "call"]);
}

#[tokio::test]
async fn given_a_call_without_a_call_id_when_run_then_an_id_is_generated_and_replayed_unchanged() {
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
    let step = call_stream(&hello(), &events)
        .await
        .outcome
        .into_parts()
        .1
        .expect("a step");
    let call = step.tool_calls().next().expect("call");
    assert!(call.id.as_str().starts_with("brgen_"));
    let history = vec![
        user_text("hello"),
        assistant(step.content().to_vec()),
        user(vec![result_block(
            call.id.as_str(),
            "search",
            &["ok"],
            false,
        )]),
    ];
    let encoded = dialect(ReasoningLevel::Off)
        .encode(&request(history))
        .expect("encoded");
    let body: serde_json::Value = serde_json::from_slice(encoded.body()).expect("json");
    assert_eq!(body["input"][1]["call_id"], json!(call.id.as_str()));
    assert_eq!(body["input"][2]["call_id"], json!(call.id.as_str()));
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
    let step = first.outcome.into_parts().1.expect("a step");
    let thinking = step.thinking().next().expect("thinking");
    assert_eq!(thinking.text, "**Looking** up\n\nthen answer");
    let signature = thinking.signature.as_ref().expect("signature").as_str();
    let envelope: serde_json::Value = serde_json::from_str(signature).expect("json");
    assert_eq!(
        envelope,
        json!({ "id": "rs_68a1", "encrypted_content": "gAAAAAB-enc" })
    );

    let history = vec![
        user_text("hello"),
        assistant(step.content().to_vec()),
        user(vec![result_block("call_1", "search", &["found"], false)]),
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
async fn given_a_refusal_in_plain_mode_when_run_then_the_step_has_the_refusal_stop_and_text() {
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
    let step = call_stream(&hello(), &events)
        .await
        .outcome
        .into_parts()
        .1
        .expect("a step");
    assert_eq!(step.stop_reason(), &StopReason::Refusal);
    let texts: Vec<&str> = step.text().map(|text| text.as_str()).collect();
    assert_eq!(texts, ["I can't help with that."]);
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
    let failure = call_stream(&request, &events)
        .await
        .outcome
        .into_parts()
        .1
        .expect_err("failure");
    assert_eq!(failure, ClientError::ContentBlocked);
}

#[tokio::test]
async fn given_a_structured_reply_when_run_then_the_step_holds_the_structured_value() {
    let events = vec![
        created("m"),
        message_added(0),
        part_added(0, 0, output_part()),
        text_delta(0, 0, "{\"answer\":"),
        text_delta(0, 0, "\"42\"}"),
        message_done(
            0,
            Some("completed"),
            vec![output_done_part("{\"answer\":\"42\"}")],
        ),
        completed("m", usage_object(2, 3)),
    ];
    let shape = shape(
        "answer",
        None,
        vec![parameter("answer", "", true, ParameterKind::String)],
    );
    let request = request_with(
        None,
        vec![user_text("go")],
        Vec::new(),
        OutputMode::Structured(shape),
    );
    let step = call_stream(&request, &events)
        .await
        .outcome
        .into_parts()
        .1
        .expect("a step");
    assert_eq!(step.structured().next(), Some(&json!({ "answer": "42" })));
}

#[tokio::test]
async fn given_a_structured_reply_with_a_function_call_when_run_then_the_reply_is_malformed() {
    let events = vec![
        created("m"),
        message_added(0),
        message_done(
            0,
            Some("completed"),
            vec![output_done_part("{\"answer\":\"42\"}")],
        ),
        call_added(1, "call_1", "search"),
        call_done(1, "call_1", "search", "{}", Some("completed")),
        completed("m", usage_object(2, 3)),
    ];
    let request = request_with(
        None,
        vec![user_text("go")],
        vec![tool("search", "", Vec::new())],
        OutputMode::Structured(shape("answer", None, Vec::new())),
    );
    let failure = call_stream(&request, &events)
        .await
        .outcome
        .into_parts()
        .1
        .expect_err("failure");
    assert!(matches!(failure, ClientError::MalformedResponse { .. }));
}

#[tokio::test]
async fn given_a_structured_request_answered_by_a_call_only_when_run_then_the_step_awaits_tool_results()
 {
    let events = vec![
        created("m"),
        call_added(0, "call_1", "search"),
        call_done(0, "call_1", "search", "{}", Some("completed")),
        completed("m", usage_object(2, 3)),
    ];
    let request = request_with(
        None,
        vec![user_text("go")],
        vec![tool("search", "", Vec::new())],
        OutputMode::Structured(shape("answer", None, Vec::new())),
    );
    let step = call_stream(&request, &events)
        .await
        .outcome
        .into_parts()
        .1
        .expect("a step");
    assert_eq!(step.stop_reason(), &StopReason::AwaitingToolResults);
}

#[tokio::test]
async fn given_an_incomplete_reply_with_text_when_run_then_the_stop_is_max_tokens_and_the_step_is_kept()
 {
    let events = vec![
        created("m"),
        message_added(0),
        part_added(0, 0, output_part()),
        text_delta(0, 0, "cut off"),
        incomplete(Some("max_output_tokens"), usage_object(9, 100)),
    ];
    let step = call_stream(&hello(), &events)
        .await
        .outcome
        .into_parts()
        .1
        .expect("a step");
    assert_eq!(step.stop_reason(), &StopReason::MaxTokens);
    assert_eq!(
        step.text().next().map(|text| text.as_str()),
        Some("cut off")
    );
}

#[tokio::test]
async fn given_an_incomplete_reply_with_only_reasoning_when_run_then_the_reply_is_unusable() {
    let events = vec![
        created("m"),
        reasoning_added(0),
        summary_delta(0, 0, "thinking forever"),
        incomplete(Some("max_output_tokens"), usage_object(9, 100)),
    ];
    let failure = call_stream(&hello(), &events)
        .await
        .outcome
        .into_parts()
        .1
        .expect_err("failure");
    assert_eq!(
        failure,
        ClientError::ReplyUnusable {
            stop_reason: StopReason::MaxTokens
        }
    );
}

#[tokio::test]
async fn given_an_incomplete_reply_with_a_cut_function_call_when_run_then_the_reply_is_unusable() {
    let events = vec![
        created("m"),
        call_added(0, "call_1", "search"),
        arguments_delta(0, "{\"q\":\"ru"),
        incomplete(Some("max_output_tokens"), usage_object(9, 100)),
    ];
    let failure = call_stream(&hello(), &events)
        .await
        .outcome
        .into_parts()
        .1
        .expect_err("failure");
    assert_eq!(
        failure,
        ClientError::ReplyUnusable {
            stop_reason: StopReason::MaxTokens
        }
    );
}

#[tokio::test]
async fn given_a_content_filter_incomplete_with_nothing_when_run_then_the_content_is_blocked() {
    let events = vec![
        created("m"),
        incomplete(Some("content_filter"), usage_object(9, 0)),
    ];
    let failure = call_stream(&hello(), &events)
        .await
        .outcome
        .into_parts()
        .1
        .expect_err("failure");
    assert_eq!(failure, ClientError::ContentBlocked);
}

#[tokio::test]
async fn given_the_sink_when_a_text_stream_runs_then_it_sees_start_deltas_end_and_finish_in_order()
{
    let called = call_stream(&hello(), &text_events("m")).await;
    let described: Vec<String> = called
        .sink
        .events
        .iter()
        .map(|event| format!("{event:?}"))
        .collect();
    assert!(described[0].starts_with("BlockStart"));
    assert!(described.iter().any(|line| line.contains("Hello")));
    assert!(described.last().expect("events").starts_with("Finish"));
}

#[tokio::test]
async fn given_a_stream_with_no_terminal_event_when_run_then_the_call_is_truncated() {
    let mut events = text_events("m");
    events.pop();
    let failure = call_stream(&hello(), &events)
        .await
        .outcome
        .into_parts()
        .1
        .expect_err("failure");
    assert_eq!(failure, ClientError::StreamTruncated);
}

#[tokio::test]
async fn given_a_list_ending_with_an_assistant_message_when_run_then_nothing_is_sent() {
    let messages: Vec<WireMessage> = vec![
        user_text("hi"),
        assistant(vec![AssistantBlock::Text { text: text("x") }]),
    ];
    let request = request(messages);
    let called = call_with_wire(dialect(ReasoningLevel::Off), &request, |_| {
        ok_wire(&sse(&[]))
    })
    .await;
    let failure = called.outcome.into_parts().1.expect_err("failure");
    assert!(matches!(failure, ClientError::InvalidMessages { .. }));
    assert!(called.requests.is_empty());
}
