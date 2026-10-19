use br_llm_clients_core::{ClientError, OutputMode, ReasoningLevel};
use br_llm_messages::{AssistantBlock, StopReason, WireMessage};

use crate::test_events::{
    completed, created, in_progress, incomplete, message_added, message_done, output_done_part,
    output_part, part_added, reasoning_added, reasoning_done, refusal_delta, refusal_done_part,
    refusal_part, summary_delta, text_delta, usage_object,
};
use crate::test_requests::{
    assistant, dialect, hello, request, request_with, shape, text, user_text,
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

#[tokio::test]
async fn given_a_structured_request_with_a_content_filter_incomplete_and_nothing_when_run_then_the_content_is_blocked()
 {
    let events = vec![
        created("m"),
        incomplete(Some("content_filter"), usage_object(9, 0)),
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
async fn given_an_incomplete_reply_with_a_refusal_text_when_run_then_the_step_ends_by_the_reason_alone()
 {
    let events = vec![
        created("m"),
        message_added(0),
        part_added(0, 0, refusal_part()),
        refusal_delta(0, 0, "I won't"),
        message_done(0, Some("incomplete"), vec![refusal_done_part("I won't")]),
        incomplete(Some("max_output_tokens"), usage_object(2, 3)),
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
        Some("I won't")
    );
}

#[tokio::test]
async fn given_a_blank_refusal_in_plain_mode_when_run_then_the_content_is_blocked() {
    let events = vec![
        created("m"),
        message_added(0),
        message_done(0, Some("completed"), vec![refusal_done_part("  ")]),
        completed("m", usage_object(2, 3)),
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
async fn given_a_thinking_only_completed_reply_when_run_then_the_reply_is_unusable() {
    let events = vec![
        created("m"),
        reasoning_added(0),
        summary_delta(0, 0, "only thoughts"),
        reasoning_done(0, "rs_1", Some("E"), &["only thoughts"]),
        completed("m", usage_object(2, 3)),
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
            stop_reason: StopReason::EndTurn
        }
    );
}
