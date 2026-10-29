use br_llm_clients_core::ClientError;
use br_llm_messages::{ContentBlock, ResponseDeltaKind, ResponseEnding, Usage};
use serde_json::json;

use crate::test_call::*;
use crate::test_response::{structured, texts, thinking, tool_calls};
use crate::test_streams::*;
use crate::test_streams_constructed::*;
use crate::test_thread::signature;

#[tokio::test]
async fn given_the_documented_basic_stream_when_run_then_the_response_holds_the_text_usage_model_and_id()
 {
    let called = call_stream(&hello(), BASIC).await;
    let response = called.response();
    assert_eq!(texts(response), ["Hello!"]);
    assert_eq!(response.ending(), &ResponseEnding::finished());
    assert_eq!(
        response.usage(),
        Some(&Usage::of_input_tokens(25).with_output_tokens(15))
    );
    assert_eq!(
        response.model().map(|model| model.as_str()),
        Some("claude-test-model")
    );
    assert_eq!(
        response.provider_response_id().map(|id| id.as_str()),
        Some("msg_test_basic")
    );
    assert_eq!(called.outcome.report().usage(), response.usage());
}

#[tokio::test]
async fn given_the_documented_tool_stream_when_run_then_the_response_holds_text_and_the_parsed_call()
 {
    let called = call_stream(&hello(), TOOL_USE).await;
    let response = called.response();
    assert_eq!(response.ending(), &ResponseEnding::tool_use());
    assert_eq!(
        texts(response),
        ["Okay, let's check the weather for San Francisco, CA:"]
    );
    let calls = tool_calls(response);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].id.as_deref(), Some("toolu_test_weather"));
    assert_eq!(calls[0].name, "get_weather");
    assert_eq!(calls[0].arguments, json!({"location": "San Francisco, CA"}));
    assert_eq!(
        response.usage().and_then(|usage| usage.output_tokens()),
        Some(89)
    );
}

#[tokio::test]
async fn given_the_documented_thinking_stream_when_run_then_the_response_holds_signed_thinking_and_no_usage()
 {
    let called = call_stream(&hello(), THINKING).await;
    let response = called.response();
    let thinking = thinking(response);
    assert_eq!(thinking.len(), 1);
    assert!(
        thinking[0]
            .0
            .starts_with("I need to find the GCD of 1071 and 462")
    );
    assert!(thinking[0].0.contains("1071 = 2 × 462 + 147"));
    assert!(thinking[0].0.ends_with("GCD(1071, 462) = 21."));
    assert_eq!(
        thinking[0].1,
        Some(signature(
            "EqQBCgIYAhIM1gbcDa9GJwZA2b3hGgxBdjrkzLoky3dl1pkiMOYds..."
        ))
    );
    assert_eq!(
        texts(response),
        ["The greatest common divisor of 1071 and 462 is **21**."]
    );
    assert_eq!(response.usage(), None);
    assert_eq!(called.outcome.report().usage(), None);
}

#[tokio::test]
async fn given_the_omitted_thinking_shape_when_run_then_the_thinking_has_empty_text_and_a_signature()
 {
    let called = call_stream(&hello(), OMITTED_THINKING).await;
    let thinking = thinking(called.response());
    assert_eq!(thinking.len(), 1);
    assert_eq!(thinking[0].0, "");
    assert_eq!(thinking[0].1, Some(signature("EqQBCgIYAhIM")));
}

#[tokio::test]
async fn given_a_tool_call_with_no_arguments_when_run_then_the_arguments_are_the_empty_object() {
    let called = call_stream(&hello(), EMPTY_TOOL_CALL).await;
    let response = called.response();
    assert_eq!(response.ending(), &ResponseEnding::tool_use());
    let calls = tool_calls(response);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "ping");
    assert_eq!(calls[0].arguments, json!({}));
}

#[tokio::test]
async fn given_a_cut_tool_call_with_max_tokens_when_run_then_it_is_never_read_as_a_call_with_no_arguments()
 {
    let called = call_stream(&hello(), MAX_TOKENS_IN_TOOL_CALL).await;
    assert_eq!(
        called.error(),
        &ClientError::ReplyUnusable {
            ending: ResponseEnding::output_exhaustion()
        }
    );
}

#[tokio::test]
async fn given_a_refusal_with_no_content_when_run_then_the_content_is_blocked() {
    let called = call_stream(&hello(), REFUSAL).await;
    assert_eq!(called.error(), &ClientError::ContentBlocked);
}

#[tokio::test]
async fn given_a_structured_reply_in_structured_mode_when_run_then_the_response_holds_the_value() {
    let called = call_with(plain_dialect(), &structured_hello(), |_| {
        crate::test_server::http("200 OK", &[], STRUCTURED_REPLY.as_bytes())
    })
    .await;
    assert_eq!(
        structured(called.response()),
        vec![json!({"value": "forty-two"})]
    );
}

#[tokio::test]
async fn given_a_redacted_thinking_stream_when_run_then_the_response_keeps_the_data_before_the_text()
 {
    let called = call_stream(&hello(), REDACTED_THINKING).await;
    let response = called.response();
    match response.blocks().first() {
        Some(ContentBlock::RedactedThinking(redacted)) => assert_eq!(
            redacted.material(),
            &crate::test_thread::redacted_data(
                "EmwKAhgBEgy3va3pzix/LafPsn4aDFIT2Xlxh0L5L8rLVyIwxtE3rAFBa8cr3qpP"
            )
        ),
        _ => panic!("the first block is not a redacted thinking block"),
    }
    assert_eq!(texts(response), ["Hi."]);
}

#[tokio::test]
async fn given_a_constructed_stream_with_cache_counts_when_run_then_the_last_usage_carries_them() {
    let called = call_stream(&hello(), CACHE_COUNTS).await;
    assert_eq!(
        called.response().usage(),
        Some(
            &Usage::of_input_tokens(14802)
                .with_output_tokens(510)
                .with_cache_read_input_tokens(4000)
                .with_cache_creation_input_tokens(120)
        )
    );
}

#[tokio::test]
async fn given_a_documented_stream_when_run_then_the_sink_gets_deltas_ending_with_the_completion() {
    let called = call_stream(&hello(), TOOL_USE).await;
    assert_eq!(
        called.deltas.last().map(|delta| delta.kind()),
        Some(ResponseDeltaKind::ResponseCompleted)
    );
    assert!(called.deltas.len() > 5);
}

#[tokio::test]
async fn given_a_tool_use_stop_with_text_and_no_call_when_run_then_the_ending_is_corrected_to_finished()
 {
    let stream = BASIC.replace("end_turn", "tool_use");
    let called = call_stream(&hello(), &stream).await;
    assert_eq!(called.response().ending(), &ResponseEnding::finished());
}

#[tokio::test]
async fn given_a_tool_use_stop_with_nothing_usable_when_run_then_the_reply_is_unusable() {
    let stream = REFUSAL.replace("refusal", "tool_use");
    let called = call_stream(&hello(), &stream).await;
    assert_eq!(
        called.error(),
        &ClientError::ReplyUnusable {
            ending: ResponseEnding::tool_use()
        }
    );
}

#[tokio::test]
async fn given_a_pause_turn_or_a_context_window_stop_with_text_when_run_then_the_ending_is_unclassified()
 {
    for reason in ["pause_turn", "model_context_window_exceeded"] {
        let stream = BASIC.replace("end_turn", reason);
        let called = call_stream(&hello(), &stream).await;
        assert_eq!(
            called.response().ending(),
            &ResponseEnding::unclassified(reason).expect("an ending")
        );
    }
}

#[tokio::test]
async fn given_a_stop_sequence_stop_when_run_then_the_ending_is_a_stop_sequence_with_no_matched_sequence()
 {
    let stream = BASIC.replace(
        r#""stop_reason": "end_turn", "stop_sequence":null"#,
        r#""stop_reason": "stop_sequence", "stop_sequence":"END""#,
    );
    assert!(stream.contains("\"END\""));
    let called = call_stream(&hello(), &stream).await;
    let ending = called.response().ending();
    assert_eq!(ending, &ResponseEnding::stop_sequence());
    assert_eq!(ending.matched_sequence(), None);
}

#[tokio::test]
async fn given_a_max_tokens_stop_with_text_when_run_then_the_response_is_kept_with_output_exhaustion()
 {
    let stream = BASIC.replace("end_turn", "max_tokens");
    let called = call_stream(&hello(), &stream).await;
    assert_eq!(
        called.response().ending(),
        &ResponseEnding::output_exhaustion()
    );
    assert_eq!(texts(called.response()), ["Hello!"]);
}
