use br_llm_clients_core::{
    CacheRequirement, ClientError, ClientRequest, ClientSettings, MaxOutputTokens, OutputMode,
    ParameterKind, ReasoningLevel,
};
use br_llm_messages::{ModelId, StopReason, StreamEvent, Usage};
use serde_json::{Value, json};

use crate::ChatCompletionsDialect;
use crate::run_support::Called;
use crate::test_recordings::{LENGTH, REASONING, REFUSAL, Recording, TEXT, TOOL, call_recorded};
use crate::test_support::{
    dialect, hello, options, parameter, request_of, request_with, shape, tool, user_text,
};

fn usage(input: u64, output: u64, thinking: u64) -> Usage {
    Usage {
        input_tokens: input,
        output_tokens: output,
        cache_read_tokens: Some(0),
        cache_write_tokens: None,
        thinking_tokens: Some(thinking),
    }
}

fn texts(called: &Called) -> Vec<&str> {
    called.step().text().map(|text| text.as_str()).collect()
}

fn assert_report(called: &Called, recording: Recording, model: &str) {
    let report = called.outcome.report();
    assert_eq!(report.request_id(), Some(recording.request_id()));
    assert_eq!(report.model().map(|model| model.as_str()), Some(model));
    assert_eq!(report.usage(), called.step().usage());
    assert_eq!(
        called.step().model().map(|model| model.as_str()),
        Some(model)
    );
}

#[tokio::test]
async fn given_the_recorded_text_reply_when_called_then_the_step_holds_the_text_the_usage_the_model_and_the_request_id()
 {
    let called = call_recorded(TEXT, dialect(), &hello()).await;
    assert_eq!(texts(&called), ["Hello! How are you today?"]);
    assert_eq!(called.step().stop_reason(), &StopReason::EndTurn);
    assert_eq!(called.step().usage(), Some(&usage(13, 7, 0)));
    assert_report(&called, TEXT, "gpt-4.1-nano-2025-04-14");
}

#[tokio::test]
async fn given_the_recorded_length_reply_when_called_then_the_step_is_the_cut_text_with_max_tokens()
{
    let called = call_recorded(LENGTH, dialect(), &hello()).await;
    assert_eq!(texts(&called), ["Beneath the endless"]);
    assert_eq!(called.step().stop_reason(), &StopReason::MaxTokens);
    assert_eq!(called.step().usage(), Some(&usage(15, 5, 0)));
    assert_report(&called, LENGTH, "gpt-4.1-nano-2025-04-14");
}

#[tokio::test]
async fn given_the_recorded_tool_reply_when_called_then_the_step_holds_the_call_and_awaits_the_result()
 {
    let weather = tool(
        "get_weather",
        "Get the weather of a city",
        vec![parameter("city", "The city", true, ParameterKind::String)],
    );
    let request = request_with(
        None,
        vec![user_text("What is the weather in Paris?")],
        vec![weather],
        OutputMode::Plain,
    );
    let called = call_recorded(TOOL, dialect(), &request).await;
    let calls: Vec<_> = called.step().tool_calls().collect();
    assert_eq!(calls.len(), 1);
    let call = calls.first().expect("a call");
    assert_eq!(call.id.as_str(), "call_ZJ4D7BK3ZGpJHd4vnCNAj22l");
    assert_eq!(call.name.as_str(), "get_weather");
    assert_eq!(call.arguments, json!({"city": "Paris"}));
    assert_eq!(
        called.step().stop_reason(),
        &StopReason::AwaitingToolResults
    );
    assert_eq!(called.step().usage(), Some(&usage(55, 30, 0)));
    assert!(texts(&called).is_empty());
    assert_report(&called, TOOL, "gpt-4.1-nano-2025-04-14");
}

fn reasoning_dialect() -> ChatCompletionsDialect {
    let settings = ClientSettings::new(
        ModelId::new("gpt-5.4-mini").expect("model"),
        ReasoningLevel::Low,
        CacheRequirement::NotRequired,
        MaxOutputTokens::new(512).expect("tokens"),
    );
    ChatCompletionsDialect::new(settings, options()).expect("dialect")
}

#[tokio::test]
async fn given_the_recorded_reasoning_reply_when_called_at_low_then_the_request_asks_low_and_the_step_counts_the_reasoning_tokens_without_thinking_text()
 {
    let request = request_of(vec![user_text("What is 17 * 23?")]);
    let called = call_recorded(REASONING, reasoning_dialect(), &request).await;
    let sent: Value =
        serde_json::from_slice(&called.server.requests()[0].body).expect("a json body");
    assert_eq!(sent["reasoning_effort"], json!("low"));
    assert_eq!(sent["model"], json!("gpt-5.4-mini"));
    assert_eq!(texts(&called), ["391"]);
    assert_eq!(called.step().usage(), Some(&usage(20, 21, 11)));
    assert_eq!(called.step().thinking().count(), 0);
    assert_report(&called, REASONING, "gpt-5.4-mini-2026-03-17");
}

fn refusal_request() -> ClientRequest {
    let output = shape(
        "answer",
        None,
        vec![parameter(
            "answer",
            "The answer",
            true,
            ParameterKind::String,
        )],
    );
    request_with(
        None,
        vec![user_text("Do something harmful")],
        Vec::new(),
        output,
    )
}

#[tokio::test]
async fn given_the_recorded_refusal_when_called_with_a_plain_output_then_the_step_is_a_refusal_holding_the_refusal_text()
 {
    let called = call_recorded(REFUSAL, dialect(), &hello()).await;
    assert_eq!(called.step().stop_reason(), &StopReason::Refusal);
    assert_eq!(
        texts(&called),
        ["I'm sorry, but I can't assist with that request."]
    );
    assert_eq!(called.step().usage(), Some(&usage(50, 12, 0)));
    assert_report(&called, REFUSAL, "gpt-4.1-mini-2025-04-14");
}

#[tokio::test]
async fn given_the_recorded_refusal_when_called_with_the_recorded_structured_output_then_the_call_is_content_blocked_and_the_sink_still_saw_the_refusal_text()
 {
    let called = call_recorded(REFUSAL, dialect(), &refusal_request()).await;
    assert_eq!(called.error(), &ClientError::ContentBlocked);
    let streamed: String = called
        .events
        .iter()
        .filter_map(|event| match event {
            StreamEvent::TextDelta { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(streamed, "I'm sorry, but I can't assist with that request.");
    let report = called.outcome.report();
    assert_eq!(report.request_id(), Some(REFUSAL.request_id()));
    assert_eq!(report.usage(), Some(&usage(50, 12, 0)));
}
