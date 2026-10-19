use br_llm_clients_core::ClientError;
use br_llm_messages::{AssistantBlock, StopReason, Usage};
use serde_json::json;

use crate::test_call::*;
use crate::test_streams::*;
use crate::test_streams_constructed::*;

fn texts(blocks: &[AssistantBlock]) -> Vec<String> {
    blocks
        .iter()
        .filter_map(|block| block.as_text().map(|text| text.as_str().to_owned()))
        .collect()
}

#[tokio::test]
async fn given_the_documented_basic_stream_when_run_then_the_step_holds_the_text_usage_and_model() {
    let called = call_stream(&hello(), BASIC).await;
    let step = called.step();
    assert_eq!(texts(step.content()), ["Hello!"]);
    assert_eq!(step.stop_reason(), &StopReason::EndTurn);
    assert_eq!(
        step.usage(),
        Some(&Usage {
            input_tokens: 25,
            output_tokens: 15,
            cache_read_tokens: None,
            cache_write_tokens: None,
            thinking_tokens: None,
        })
    );
    assert_eq!(
        step.model().map(|model| model.as_str()),
        Some("claude-test-model")
    );
    assert_eq!(called.outcome.report().usage(), step.usage());
}

#[tokio::test]
async fn given_the_documented_tool_stream_when_run_then_the_step_holds_text_and_the_parsed_call() {
    let called = call_stream(&hello(), TOOL_USE).await;
    let step = called.step();
    assert_eq!(step.stop_reason(), &StopReason::AwaitingToolResults);
    assert_eq!(
        texts(step.content()),
        ["Okay, let's check the weather for San Francisco, CA:"]
    );
    let calls: Vec<_> = step.tool_calls().collect();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].id.as_str(), "toolu_01T1x1fJ34qAmk2tNTrN7Up6");
    assert_eq!(calls[0].name.as_str(), "get_weather");
    assert_eq!(calls[0].arguments, json!({"location": "San Francisco, CA"}));
    assert_eq!(step.usage().map(|usage| usage.output_tokens), Some(89));
}

#[tokio::test]
async fn given_the_documented_thinking_stream_when_run_then_the_step_holds_signed_thinking_and_no_usage()
 {
    let called = call_stream(&hello(), THINKING).await;
    let step = called.step();
    let thinking: Vec<_> = step.thinking().collect();
    assert_eq!(thinking.len(), 1);
    assert!(
        thinking[0]
            .text
            .starts_with("I need to find the GCD of 1071 and 462")
    );
    assert!(thinking[0].text.contains("1071 = 2 × 462 + 147"));
    assert!(thinking[0].text.ends_with("GCD(1071, 462) = 21."));
    assert_eq!(
        thinking[0]
            .signature
            .as_ref()
            .map(|signature| signature.as_str()),
        Some("EqQBCgIYAhIM1gbcDa9GJwZA2b3hGgxBdjrkzLoky3dl1pkiMOYds...")
    );
    assert_eq!(
        texts(step.content()),
        ["The greatest common divisor of 1071 and 462 is **21**."]
    );
    assert_eq!(step.usage(), None);
    assert_eq!(called.outcome.report().usage(), None);
}

#[tokio::test]
async fn given_the_omitted_thinking_shape_when_run_then_the_thinking_has_empty_text_and_a_signature()
 {
    let called = call_stream(&hello(), OMITTED_THINKING).await;
    let thinking: Vec<_> = called.step().thinking().collect();
    assert_eq!(thinking.len(), 1);
    assert_eq!(thinking[0].text, "");
    assert!(thinking[0].signature.is_some());
}

#[tokio::test]
async fn given_a_tool_call_with_no_arguments_when_run_then_the_arguments_are_the_empty_object() {
    let called = call_stream(&hello(), EMPTY_TOOL_CALL).await;
    let step = called.step();
    assert_eq!(step.stop_reason(), &StopReason::AwaitingToolResults);
    let calls: Vec<_> = step.tool_calls().collect();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name.as_str(), "ping");
    assert_eq!(calls[0].arguments, json!({}));
}

#[tokio::test]
async fn given_a_cut_tool_call_with_max_tokens_when_run_then_it_is_never_read_as_a_call_with_no_arguments()
 {
    let called = call_stream(&hello(), MAX_TOKENS_IN_TOOL_CALL).await;
    assert_eq!(
        called.error(),
        &ClientError::ReplyUnusable {
            stop_reason: StopReason::MaxTokens
        }
    );
}

#[tokio::test]
async fn given_a_refusal_with_no_content_when_run_then_the_content_is_blocked() {
    let called = call_stream(&hello(), REFUSAL).await;
    assert_eq!(called.error(), &ClientError::ContentBlocked);
}

#[tokio::test]
async fn given_a_structured_reply_in_structured_mode_when_run_then_the_step_holds_the_value() {
    let called = call_with(plain_dialect(), &structured_hello(), |_| {
        crate::test_server::http("200 OK", &[], STRUCTURED_REPLY.as_bytes())
    })
    .await;
    let values: Vec<_> = called.step().structured().cloned().collect();
    assert_eq!(values, vec![json!({"value": "forty-two"})]);
}

#[tokio::test]
async fn given_a_redacted_thinking_stream_when_run_then_the_step_keeps_the_data_before_the_text() {
    let called = call_stream(&hello(), REDACTED_THINKING).await;
    let content = called.step().content();
    assert!(matches!(
        content.first(),
        Some(AssistantBlock::RedactedThinking(_))
    ));
    assert_eq!(texts(content), ["Hi."]);
}

#[tokio::test]
async fn given_a_constructed_stream_with_cache_counts_when_run_then_the_last_usage_carries_them() {
    let called = call_stream(&hello(), CACHE_COUNTS).await;
    assert_eq!(
        called.step().usage(),
        Some(&Usage {
            input_tokens: 14802,
            output_tokens: 510,
            cache_read_tokens: Some(4000),
            cache_write_tokens: Some(120),
            thinking_tokens: None,
        })
    );
}

#[tokio::test]
async fn given_a_documented_stream_when_run_then_the_sink_gets_events_ending_with_the_finish() {
    let called = call_stream(&hello(), TOOL_USE).await;
    assert!(matches!(
        called.events.last(),
        Some(br_llm_messages::StreamEvent::Finish { .. })
    ));
    assert!(called.events.len() > 5);
}

#[tokio::test]
async fn given_a_tool_use_stop_with_text_and_no_call_when_run_then_the_stop_is_corrected_to_end_turn()
 {
    let stream = BASIC.replace("end_turn", "tool_use");
    let called = call_stream(&hello(), &stream).await;
    assert_eq!(called.step().stop_reason(), &StopReason::EndTurn);
}

#[tokio::test]
async fn given_a_tool_use_stop_with_nothing_usable_when_run_then_the_reply_is_unusable() {
    let stream = REFUSAL.replace("refusal", "tool_use");
    let called = call_stream(&hello(), &stream).await;
    assert_eq!(
        called.error(),
        &ClientError::ReplyUnusable {
            stop_reason: StopReason::AwaitingToolResults
        }
    );
}

#[tokio::test]
async fn given_a_pause_turn_with_text_when_run_then_the_stop_reason_is_other() {
    let stream = BASIC.replace("end_turn", "pause_turn");
    let called = call_stream(&hello(), &stream).await;
    assert_eq!(
        called.step().stop_reason(),
        &StopReason::Other {
            reason: "pause_turn".to_owned()
        }
    );
}

#[tokio::test]
async fn given_a_max_tokens_stop_with_text_when_run_then_the_step_is_kept_with_max_tokens() {
    let stream = BASIC.replace("end_turn", "max_tokens");
    let called = call_stream(&hello(), &stream).await;
    assert_eq!(called.step().stop_reason(), &StopReason::MaxTokens);
    assert_eq!(texts(called.step().content()), ["Hello!"]);
}
