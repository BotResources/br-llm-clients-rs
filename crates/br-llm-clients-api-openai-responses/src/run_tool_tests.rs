use br_llm_clients_core::{ClientError, OutputMode, ParameterKind};
use br_llm_messages::{ResponseDelta, ResponseDeltaKind, ResponseEnding};

use crate::test_events::{
    arguments_delta, call_added, call_done, completed, created, in_progress, incomplete,
    message_added, message_done, output_done_part, output_part, part_added, reasoning_added,
    summary_delta, text_delta, usage_object,
};
use crate::test_requests::{hello, parameter, request_with, shape, tool, user_text};
use crate::test_run::{call_stream, kinds, texts};

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
async fn given_a_structured_reply_when_run_then_the_reply_holds_the_structured_value() {
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
    let step = call_stream(&request, &events).await.reply();
    let values: Vec<String> = step
        .blocks()
        .iter()
        .filter_map(|block| match block {
            br_llm_messages::ContentBlock::StructuredContent(found) => {
                Some(found.value().to_json_string().expect("json"))
            }
            _ => None,
        })
        .collect();
    assert_eq!(values, ["{\"answer\":\"42\"}"]);
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
    let failure = call_stream(&request, &events).await.failure();
    assert!(matches!(failure, ClientError::MalformedResponse { .. }));
}

#[tokio::test]
async fn given_a_structured_request_answered_by_a_call_only_when_run_then_the_reply_uses_a_tool() {
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
    let step = call_stream(&request, &events).await.reply();
    assert_eq!(step.ending(), &ResponseEnding::tool_use());
    assert_eq!(kinds(&step), ["call"]);
}

#[tokio::test]
async fn given_an_incomplete_reply_with_text_when_run_then_the_ending_is_output_exhaustion_and_the_reply_is_kept()
 {
    let events = vec![
        created("m"),
        message_added(0),
        part_added(0, 0, output_part()),
        text_delta(0, 0, "cut off"),
        incomplete(Some("max_output_tokens"), usage_object(9, 100)),
    ];
    let step = call_stream(&hello(), &events).await.reply();
    assert_eq!(step.ending(), &ResponseEnding::output_exhaustion());
    assert_eq!(texts(&step).first().map(String::as_str), Some("cut off"));
}

#[tokio::test]
async fn given_an_incomplete_reply_with_only_reasoning_when_run_then_the_reply_is_unusable() {
    let events = vec![
        created("m"),
        reasoning_added(0),
        summary_delta(0, 0, "thinking forever"),
        incomplete(Some("max_output_tokens"), usage_object(9, 100)),
    ];
    let failure = call_stream(&hello(), &events).await.failure();
    assert_eq!(
        failure,
        ClientError::ReplyUnusable {
            ending: ResponseEnding::output_exhaustion()
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
    let failure = call_stream(&hello(), &events).await.failure();
    assert_eq!(
        failure,
        ClientError::ReplyUnusable {
            ending: ResponseEnding::output_exhaustion()
        }
    );
}

#[tokio::test]
async fn given_a_content_filter_incomplete_with_nothing_when_run_then_the_content_is_blocked() {
    let events = vec![
        created("m"),
        incomplete(Some("content_filter"), usage_object(9, 0)),
    ];
    let failure = call_stream(&hello(), &events).await.failure();
    assert_eq!(failure, ClientError::ContentBlocked);
}

#[tokio::test]
async fn given_the_sink_when_a_text_stream_runs_then_it_sees_started_first_completed_last_and_folds_to_the_reply()
 {
    let called = call_stream(&hello(), &text_events("m")).await;
    let kinds: Vec<ResponseDeltaKind> =
        called.sink.deltas.iter().map(ResponseDelta::kind).collect();
    assert_eq!(kinds.first(), Some(&ResponseDeltaKind::ResponseStarted));
    assert_eq!(kinds.last(), Some(&ResponseDeltaKind::ResponseCompleted));
    assert!(kinds.contains(&ResponseDeltaKind::BlockStarted));
    let mut draft = br_llm_messages::ResponseDraft::new(
        crate::test_history::reply_turn(),
        crate::test_history::reply_entry(),
    );
    for delta in &called.sink.deltas {
        draft.apply(delta).expect("the draft accepts the delta");
    }
    let reply = called.outcome.into_parts().1.expect("a reply");
    assert_eq!(draft.into_response(), Some(reply));
}
