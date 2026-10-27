use br_llm_clients_core::ClientError;
use br_llm_messages::{ContentBlock, ResponseDraft, ResponseEnding, Usage};

use crate::test_decoder::EndScript;
use crate::test_fixtures::user_request;
use crate::test_ids::{reply_entry, reply_turn};
use crate::test_requests::structured_request;
use crate::test_stream::{Driven, body, dialect_ending, drive_one, kinds};

async fn run(lines: &[&str]) -> Driven {
    let dialect = dialect_ending(EndScript::Events(Vec::new()));
    drive_one(&dialect, &user_request(), body(&[lines])).await
}

#[tokio::test]
async fn given_usage_model_and_response_id_when_the_reply_ends_then_the_response_carries_the_last_ones()
 {
    let driven = run(&[
        "usage 1 1",
        "start a text",
        "text a hi",
        "end a",
        "stop finished",
        "usage 7 9",
        "model first",
        "model last",
        "rid r1",
        "rid r2",
    ])
    .await;
    let response = driven.result.expect("a response");
    assert_eq!(
        response.usage(),
        Some(&Usage::of_input_tokens(7).with_output_tokens(9))
    );
    assert_eq!(response.model().map(|model| model.as_str()), Some("last"));
    assert_eq!(
        response.provider_response_id().map(|id| id.as_str()),
        Some("r2")
    );
}

#[tokio::test]
async fn given_a_usage_event_with_one_count_when_the_reply_ends_then_the_response_carries_it_as_it_is()
 {
    let driven = run(&[
        "usage 5",
        "start a text",
        "text a hi",
        "end a",
        "stop finished",
    ])
    .await;
    let response = driven.result.expect("a response");
    assert_eq!(response.usage(), Some(&Usage::of_input_tokens(5)));
    assert!(kinds(&driven.sink.deltas).contains(&"meta usage=Some(5)/None".to_owned()));
}

#[tokio::test]
async fn given_no_model_event_and_no_answered_model_when_the_reply_ends_then_the_response_has_no_model()
 {
    let driven = run(&["start a text", "text a hi", "end a", "stop finished"]).await;
    let response = driven.result.expect("a response");
    assert_eq!(response.model(), None);
    assert_eq!(response.usage(), None);
    assert_eq!(response.provider_response_id(), None);
}

#[tokio::test]
async fn given_a_reply_when_built_then_the_first_delta_starts_it_and_the_last_completes_it_and_no_time_is_set()
 {
    let driven = run(&["start a text", "text a hi", "end a", "stop finished"]).await;
    let response = driven.result.expect("a response");
    let lines = kinds(&driven.sink.deltas);
    assert_eq!(lines.first().map(String::as_str), Some("started"));
    assert_eq!(lines.last().map(String::as_str), Some("done Finished"));
    assert_eq!(
        lines.iter().filter(|line| line.starts_with("done")).count(),
        1
    );
    assert_eq!(response.observed_at(), None);
    for delta in &driven.sink.deltas {
        assert_eq!(delta.turn(), reply_turn());
        assert_eq!(delta.entry(), reply_entry());
    }
}

#[tokio::test]
async fn given_tool_calls_with_and_without_a_provider_id_when_the_reply_ends_then_the_external_id_is_carried_as_received_or_absent()
 {
    let driven = run(&[
        "start a tool search Call-1",
        "args a {}",
        "end a",
        "start b tool search",
        "args b {}",
        "end b",
        "start c tool search Call-1",
        "args c {}",
        "end c",
        "stop tool_use",
    ])
    .await;
    let response = driven.result.expect("a response");
    let ids: Vec<_> = response
        .runtime_calls()
        .map(|call| call.external_id().map(|id| id.as_str().to_owned()))
        .collect();
    assert_eq!(
        ids,
        vec![Some("Call-1".to_owned()), None, Some("Call-1".to_owned())]
    );
}

#[tokio::test]
async fn given_the_structured_mode_when_the_reply_holds_a_value_then_the_response_is_that_value() {
    let dialect = dialect_ending(EndScript::Events(Vec::new()));
    let bytes = body(&[&[
        "start a structured",
        "struct a {\"ok\":true}",
        "end a",
        "stop finished",
    ]]);
    let driven = drive_one(&dialect, &structured_request(false), bytes).await;
    let response = driven.result.expect("a response");
    assert!(matches!(
        response.blocks(),
        [ContentBlock::StructuredContent(_)]
    ));
    let text = body(&[&["start a text", "text a hi", "end a", "stop finished"]]);
    let refused = drive_one(&dialect, &structured_request(false), text).await;
    assert!(matches!(
        refused.result,
        Err(ClientError::MalformedResponse { .. })
    ));
}

#[tokio::test]
async fn given_a_response_when_built_then_folding_the_deltas_the_sink_got_gives_an_equal_response()
{
    let driven = run(&[
        "start a thinking",
        "start b text",
        "text b   ",
        "text b Hello",
        "think a hm",
        "sig a fmt/1 s1",
        "end b",
        "end a",
        "start c tool search call_1",
        "args c {\"x\":1}",
        "end c",
        "model m",
        "stop finished",
        "usage 2 3",
        "rid r",
    ])
    .await;
    let response = driven.result.expect("a response");
    let mut draft = ResponseDraft::new(reply_turn(), reply_entry());
    for delta in &driven.sink.deltas {
        draft.apply(delta).expect("accepted");
    }
    assert_eq!(draft.into_response(), Some(response));
}

#[tokio::test]
async fn given_a_reply_with_a_cut_ending_and_a_complete_content_when_built_then_the_ending_is_the_one_of_the_provider()
 {
    let driven = run(&["start a text", "text a hi", "end a", "stop exhaust"]).await;
    let response = driven.result.expect("a response");
    assert_eq!(response.ending(), &ResponseEnding::output_exhaustion());
}
