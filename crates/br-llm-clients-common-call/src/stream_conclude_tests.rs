use br_llm_clients_core::ClientError;
use br_llm_messages::{ContentBlock, ResponseEnding};

use crate::test_decoder::EndScript;
use crate::test_fixtures::user_request;
use crate::test_stream::{Driven, body, dialect_ending, drive_one, kinds};

async fn run(lines: &[&str]) -> Driven {
    let dialect = dialect_ending(EndScript::Events(Vec::new()));
    drive_one(&dialect, &user_request(), body(&[lines])).await
}

#[tokio::test]
async fn given_a_structured_block_never_closed_when_the_stream_ends_then_the_reply_is_malformed_and_only_the_head_deltas_stand()
 {
    let driven = run(&[
        "start a thinking",
        "think a t",
        "sig a fmt/1 sg",
        "start b redacted",
        "redact b fmt/1 zz",
        "start c structured",
        "struct c {\"k\":1}",
        "start d text",
        "text d Hi",
        "stop finished",
    ])
    .await;
    let error = driven.result.clone().expect_err("structured is not closed");
    assert!(
        matches!(error, ClientError::MalformedResponse { .. }),
        "{error:?}"
    );
    assert_eq!(
        kinds(&driven.sink.deltas),
        vec![
            "started",
            "start 1 thinking",
            "think 1 \"t\"",
            "replay 1 fmt/1 \"sg\""
        ]
    );
}

#[tokio::test]
async fn given_text_thinking_and_redacted_blocks_open_at_the_stop_when_the_stream_ends_then_they_are_closed_in_order_before_the_completion()
 {
    let driven = run(&[
        "start a thinking",
        "think a t",
        "start b redacted",
        "redact b fmt/1 zz",
        "start c text",
        "text c Hi",
        "stop finished",
    ])
    .await;
    let response = driven.result.expect("a response");
    assert_eq!(response.blocks().len(), 3);
    assert_eq!(
        kinds(&driven.sink.deltas),
        vec![
            "started",
            "start 1 thinking",
            "think 1 \"t\"",
            "final 1",
            "start 2 redacted",
            "replay 2 fmt/1 \"zz\"",
            "final 2",
            "start 3 text",
            "text 3 \"Hi\"",
            "final 3",
            "done Finished",
        ]
    );
}

#[tokio::test]
async fn given_a_block_closed_by_the_provider_with_json_when_the_reply_ends_then_the_response_holds_the_value()
 {
    let driven = run(&[
        "start a structured",
        "struct a {\"k\":",
        "struct a 1}",
        "end a",
        "stop finished",
    ])
    .await;
    let response = driven.result.expect("a response");
    let [ContentBlock::StructuredContent(block)] = response.blocks() else {
        panic!("not one structured block");
    };
    assert_eq!(block.value().to_json_string().expect("json"), "{\"k\":1}");
}

#[tokio::test]
async fn given_a_blank_text_block_open_at_the_stop_when_the_stream_ends_then_it_is_dropped_and_the_next_block_is_released()
 {
    let driven = run(&[
        "start a text",
        "text a  ",
        "start b text",
        "text b t",
        "stop finished",
    ])
    .await;
    assert!(driven.result.is_ok());
    assert_eq!(
        kinds(&driven.sink.deltas),
        vec![
            "started",
            "start 1 text",
            "text 1 \"t\"",
            "final 1",
            "done Finished"
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
        "stop exhaust",
    ])
    .await;
    assert_eq!(
        driven.result.clone(),
        Err(ClientError::ReplyUnusable {
            ending: ResponseEnding::output_exhaustion()
        })
    );
    assert_eq!(
        kinds(&driven.sink.deltas),
        vec!["started", "start 1 text", "text 1 \"hello\""]
    );
}

#[tokio::test]
async fn given_a_block_the_provider_never_closed_when_the_ending_is_a_filter_then_the_content_is_blocked()
 {
    let driven = run(&["start a tool search id1", "args a {}", "stop filter"]).await;
    assert_eq!(driven.result, Err(ClientError::ContentBlocked));
    assert_eq!(
        kinds(&driven.sink.deltas),
        vec!["started", "start 1 call search id1", "json 1 \"{}\""]
    );
}

#[tokio::test]
async fn given_a_closing_the_draft_refuses_when_the_stream_ends_then_the_reply_is_malformed_and_the_held_deltas_are_never_released()
 {
    let driven = run(&[
        "start a redacted",
        "start b text",
        "text b waiting",
        "stop finished",
    ])
    .await;
    assert!(matches!(
        driven.result,
        Err(ClientError::MalformedResponse { .. })
    ));
    assert_eq!(
        kinds(&driven.sink.deltas),
        vec!["started", "start 1 redacted"]
    );
}

#[tokio::test]
async fn given_a_redacted_block_without_data_closed_by_the_provider_when_read_then_the_reply_is_malformed()
 {
    let driven = run(&[
        "start t text",
        "text t hi",
        "end t",
        "start a redacted",
        "end a",
        "stop finished",
    ])
    .await;
    assert!(matches!(
        driven.result,
        Err(ClientError::MalformedResponse { .. })
    ));
}

#[tokio::test]
async fn given_a_tool_call_with_a_wrong_ending_when_the_reply_ends_then_the_response_and_the_completion_carry_the_corrected_one()
 {
    let driven = run(&[
        "start t tool search id1",
        "args t {}",
        "end t",
        "stop finished",
    ])
    .await;
    let response = driven.result.expect("a response");
    assert_eq!(response.ending(), &ResponseEnding::tool_use());
    assert_eq!(
        kinds(&driven.sink.deltas).last().map(String::as_str),
        Some("done ToolUse")
    );
    let plain = run(&["start a text", "text a hi", "end a", "stop tool_use"]).await;
    assert_eq!(
        plain.result.expect("a response").ending(),
        &ResponseEnding::finished()
    );
}

#[tokio::test]
async fn given_a_reply_that_ends_for_a_filter_with_nothing_when_read_then_the_content_is_blocked_after_the_deltas_stand()
 {
    let driven = run(&["start a thinking", "think a hmm", "end a", "stop refusal"]).await;
    assert_eq!(driven.result, Err(ClientError::ContentBlocked));
    assert_eq!(
        kinds(&driven.sink.deltas),
        vec!["started", "start 1 thinking", "think 1 \"hmm\"", "final 1"]
    );
}

#[tokio::test]
async fn given_a_reply_with_incomplete_json_closed_by_the_provider_when_the_ending_is_a_cut_then_the_deltas_stand_and_the_reply_is_unusable()
 {
    let driven = run(&[
        "start a tool search id1",
        "args a {\"q\":",
        "end a",
        "start b text",
        "text b after",
        "end b",
        "stop exhaust",
    ])
    .await;
    assert_eq!(
        driven.result,
        Err(ClientError::ReplyUnusable {
            ending: ResponseEnding::output_exhaustion()
        })
    );
    assert_eq!(
        kinds(&driven.sink.deltas),
        vec![
            "started",
            "start 1 call search id1",
            "json 1 \"{\\\"q\\\":\"",
            "start 2 text",
            "text 2 \"after\"",
            "final 2"
        ]
    );
}

#[tokio::test]
async fn given_json_nested_deeper_than_the_draft_accepts_when_the_provider_closes_the_block_then_the_reply_is_malformed()
 {
    let deep = format!("{}1{}", "[".repeat(70), "]".repeat(70));
    let line = format!("struct a {deep}");
    let driven = run(&["start a structured", &line, "end a", "stop finished"]).await;
    assert!(matches!(
        driven.result,
        Err(ClientError::MalformedResponse { .. })
    ));
}
