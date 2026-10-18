use br_llm_clients_core::ClientError;
use br_llm_messages::StopReason;

use crate::test_decoder::EndScript;
use crate::test_fixtures::user_request;
use crate::test_stream::{Driven, body, dialect_ending, drive_one, kinds};

async fn run(lines: &[&str]) -> Driven {
    let dialect = dialect_ending(EndScript::Events(Vec::new()));
    drive_one(&dialect, &user_request(), body(&[lines])).await
}

#[tokio::test]
async fn given_a_structured_block_never_closed_when_the_stream_ends_then_the_reply_is_malformed_and_only_the_head_events_stand()
 {
    let driven = run(&[
        "start a thinking",
        "think a t",
        "sig a sg",
        "start b redacted",
        "redact b zz",
        "start c structured",
        "struct c {\"k\":1}",
        "start d text",
        "text d Hi",
        "stop end_turn",
    ])
    .await;
    let error = driven.result.clone().expect_err("structured is not closed");
    assert!(
        matches!(error, ClientError::MalformedResponse { .. }),
        "{error:?}"
    );
    assert_eq!(
        kinds(&driven.sink.events),
        vec!["start 0 Thinking", "think 0 \"t\"", "sig 0 \"sg\""]
    );
}

#[tokio::test]
async fn given_text_thinking_and_redacted_blocks_open_at_the_stop_when_the_stream_ends_then_they_are_closed_in_order_before_the_finish()
 {
    let driven = run(&[
        "start a thinking",
        "think a t",
        "start b redacted",
        "redact b zz",
        "start c text",
        "text c Hi",
        "stop end_turn",
    ])
    .await;
    let step = driven.result.expect("a step");
    assert_eq!(step.content().len(), 3);
    assert_eq!(
        kinds(&driven.sink.events),
        vec![
            "start 0 Thinking",
            "think 0 \"t\"",
            "end 0",
            "start 1 RedactedThinking",
            "redact 1 \"zz\"",
            "end 1",
            "start 2 Text",
            "text 2 \"Hi\"",
            "end 2",
            "finish end_turn false",
        ]
    );
}

#[tokio::test]
async fn given_a_block_closed_by_the_provider_with_json_when_the_reply_ends_then_the_step_holds_the_value()
 {
    let driven = run(&[
        "start a structured",
        "struct a {\"k\":",
        "struct a 1}",
        "end a",
        "stop end_turn",
    ])
    .await;
    let step = driven.result.expect("a step");
    assert_eq!(step.structured().count(), 1);
    assert_eq!(step.structured().next(), Some(&serde_json::json!({"k": 1})));
}

#[tokio::test]
async fn given_a_blank_text_block_open_at_the_stop_when_the_stream_ends_then_it_is_dropped_and_the_indexes_stay_consecutive()
 {
    let driven = run(&[
        "start a text",
        "text a  ",
        "start b text",
        "text b t",
        "stop end_turn",
    ])
    .await;
    assert!(driven.result.is_ok());
    assert_eq!(
        kinds(&driven.sink.events),
        vec![
            "start 0 Text",
            "text 0 \"t\"",
            "end 0",
            "finish end_turn false"
        ]
    );
}

#[tokio::test]
async fn given_a_judgement_that_fails_when_the_stream_ends_then_nothing_more_reaches_the_sink() {
    let driven = run(&[
        "start a text",
        "text a hello",
        "start b tool search id1",
        "args b {\"q\":",
        "stop max",
    ])
    .await;
    assert_eq!(
        driven.result.clone(),
        Err(ClientError::ReplyUnusable {
            stop_reason: StopReason::MaxTokens
        })
    );
    assert_eq!(
        kinds(&driven.sink.events),
        vec!["start 0 Text", "text 0 \"hello\""]
    );
}

#[tokio::test]
async fn given_a_block_the_provider_never_closed_when_the_stop_is_a_filter_then_the_content_is_blocked()
 {
    let driven = run(&["start a tool search id1", "args a {}", "stop filter"]).await;
    assert_eq!(driven.result, Err(ClientError::ContentBlocked));
    assert_eq!(
        kinds(&driven.sink.events),
        vec!["call 0 id1 search", "args 0 \"{}\""]
    );
}

#[tokio::test]
async fn given_a_reply_the_step_refuses_when_the_stream_ends_then_it_is_malformed_and_no_finish_is_sent()
 {
    let driven = run(&[
        "start t tool search id1",
        "args t {}",
        "end t",
        "start x text",
        "text x late",
        "end x",
        "stop tool",
    ])
    .await;
    assert!(matches!(
        driven.result,
        Err(ClientError::MalformedResponse { .. })
    ));
    assert!(
        !kinds(&driven.sink.events)
            .iter()
            .any(|event| event.starts_with("finish"))
    );
}

#[tokio::test]
async fn given_a_redacted_block_without_data_when_the_stream_ends_then_it_is_malformed() {
    let driven = run(&[
        "start t text",
        "text t hi",
        "end t",
        "start a redacted",
        "end a",
        "stop end_turn",
    ])
    .await;
    assert!(matches!(
        driven.result,
        Err(ClientError::MalformedResponse { .. })
    ));
}

#[tokio::test]
async fn given_a_tool_call_with_a_wrong_stop_reason_when_the_reply_ends_then_the_step_and_the_finish_carry_the_corrected_one()
 {
    let driven = run(&[
        "start t tool search id1",
        "args t {}",
        "end t",
        "stop end_turn",
    ])
    .await;
    let step = driven.result.expect("a step");
    assert_eq!(step.stop_reason(), &StopReason::AwaitingToolResults);
    assert_eq!(
        kinds(&driven.sink.events).last().map(String::as_str),
        Some("finish awaiting_tool_results false")
    );
    let plain = run(&["start a text", "text a hi", "end a", "stop tool"]).await;
    assert_eq!(
        plain.result.expect("a step").stop_reason(),
        &StopReason::EndTurn
    );
}

#[tokio::test]
async fn given_a_reply_that_stops_for_a_filter_with_nothing_when_read_then_the_content_is_blocked_after_the_events_stand()
 {
    let driven = run(&["start a thinking", "think a hmm", "end a", "stop refusal"]).await;
    assert_eq!(driven.result, Err(ClientError::ContentBlocked));
    assert_eq!(
        kinds(&driven.sink.events),
        vec!["start 0 Thinking", "think 0 \"hmm\"", "end 0"]
    );
}
