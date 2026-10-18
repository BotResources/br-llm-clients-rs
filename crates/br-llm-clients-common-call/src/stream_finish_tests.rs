use br_llm_clients_core::ClientError;
use br_llm_messages::{AssistantBlock, StopReason, Usage};

use crate::test_decoder::EndScript;
use crate::test_fixtures::user_request;
use crate::test_requests::{request_with_calls, structured_request};
use crate::test_stream::{Driven, body, dialect_ending, drive_one};

async fn run(lines: &[&str]) -> Driven {
    let dialect = dialect_ending(EndScript::Events(Vec::new()));
    drive_one(&dialect, &user_request(), body(&[lines])).await
}

#[tokio::test]
async fn given_usage_and_model_when_the_reply_ends_then_the_step_and_the_finish_carry_the_last_ones()
 {
    let driven = run(&[
        "usage 1 1",
        "start a text",
        "text a hi",
        "end a",
        "stop end_turn",
        "usage 7 9",
        "model first",
        "model last",
    ])
    .await;
    let step = driven.result.expect("a step");
    let usage = Usage {
        input_tokens: 7,
        output_tokens: 9,
        ..Usage::default()
    };
    assert_eq!(step.usage(), Some(&usage));
    assert_eq!(step.model().map(|model| model.as_str()), Some("last"));
    assert_eq!(
        driven.sink.events.last(),
        Some(&br_llm_messages::StreamEvent::Finish {
            stop_reason: StopReason::EndTurn,
            usage: Some(usage)
        })
    );
}

#[tokio::test]
async fn given_no_model_event_when_the_reply_ends_then_the_step_holds_the_model_of_the_spec() {
    let driven = run(&["start a text", "text a hi", "end a", "stop end_turn"]).await;
    let step = driven.result.expect("a step");
    assert_eq!(step.model().map(|model| model.as_str()), Some("test-model"));
    assert_eq!(step.usage(), None);
}

#[tokio::test]
async fn given_a_tool_call_without_an_id_when_the_reply_ends_then_the_step_holds_a_generated_id_other_than_the_request_ones()
 {
    let dialect = dialect_ending(EndScript::Events(Vec::new()));
    let request = request_with_calls(&["brgen_0"]);
    let bytes = body(&[&["start t tool search", "args t {}", "end t", "stop tool"]]);
    let driven = drive_one(&dialect, &request, bytes).await;
    let step = driven.result.expect("a step");
    let call = step.tool_calls().next().expect("a call");
    assert!(crate::generated_id::is_generated_id(call.id.as_str()));
    assert_ne!(call.id.as_str(), "brgen_0");
}

#[tokio::test]
async fn given_the_structured_mode_when_the_reply_holds_a_value_then_the_step_is_that_value() {
    let dialect = dialect_ending(EndScript::Events(Vec::new()));
    let bytes = body(&[&[
        "start a structured",
        "struct a {\"ok\":true}",
        "end a",
        "stop end_turn",
    ]]);
    let driven = drive_one(&dialect, &structured_request(false), bytes).await;
    let step = driven.result.expect("a step");
    assert!(matches!(
        step.content(),
        [AssistantBlock::Structured { .. }]
    ));
    let text = body(&[&["start a text", "text a hi", "end a", "stop end_turn"]]);
    let refused = drive_one(&dialect, &structured_request(false), text).await;
    assert!(matches!(
        refused.result,
        Err(ClientError::MalformedResponse { .. })
    ));
}

#[tokio::test]
async fn given_a_step_when_built_then_the_events_it_folds_are_exactly_the_events_the_sink_got() {
    let driven = run(&[
        "start a thinking",
        "start b text",
        "text b   ",
        "text b Hello",
        "think a hm",
        "end b",
        "end a",
        "start c tool search",
        "args c {\"x\":1}",
        "end c",
        "stop end_turn",
        "usage 2 3",
    ])
    .await;
    let step = driven.result.expect("a step");
    let mut draft = br_llm_messages::StepDraft::new();
    for event in &driven.sink.events {
        draft.apply(event.clone()).expect("accepted");
    }
    let model = step.model().cloned();
    assert_eq!(draft.finish(model).expect("a step"), step);
}
