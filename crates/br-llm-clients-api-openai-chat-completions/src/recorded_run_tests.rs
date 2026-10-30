use br_llm_clients_core::{
    CacheRequirement, ClientError, ClientRequest, ClientSettings, MaxOutputTokens, OutputMode,
    ParameterKind, ReasoningLevel,
};
use br_llm_messages::{ModelName, ResponseEnding, Usage};
use serde_json::{Value, json};

use crate::ChatCompletionsDialect;
use crate::run_support::{Called, appended_text, arguments_of, calls_of, texts_of, thinking_of};
use crate::test_recordings::{LENGTH, REASONING, REFUSAL, Recording, TEXT, TOOL, call_recorded};
use crate::test_support::{
    dialect, hello, options, parameter, request_of, request_with, shape, tool,
};
use crate::test_thread::user;

fn usage(input: u64, output: u64, thinking: u64) -> Usage {
    Usage::of_input_tokens(input)
        .with_output_tokens(output)
        .with_cache_read_input_tokens(0)
        .with_thinking_tokens(thinking)
}

fn texts(called: &Called) -> Vec<&str> {
    texts_of(called.response())
}

fn assert_report(called: &Called, recording: Recording, model: &str, response_id: &str) {
    assert_eq!(
        called
            .response()
            .provider_response_id()
            .map(|id| id.as_str()),
        Some(response_id)
    );
    let report = called.outcome.report();
    assert_eq!(report.request_id(), Some(recording.request_id()));
    assert_eq!(report.model().map(|model| model.as_str()), Some(model));
    assert_eq!(report.usage(), called.response().usage());
    assert_eq!(
        called.response().model().map(|model| model.as_str()),
        Some(model)
    );
}

#[tokio::test]
async fn given_the_recorded_text_reply_when_called_then_the_response_holds_the_text_the_usage_the_model_and_the_ids()
 {
    let called = call_recorded(TEXT, dialect(), &hello()).await;
    assert_eq!(texts(&called), ["Hello! How are you today?"]);
    assert_eq!(called.response().ending(), &ResponseEnding::finished());
    assert_eq!(called.response().usage(), Some(&usage(13, 7, 0)));
    assert_report(
        &called,
        TEXT,
        "gpt-4.1-nano-2025-04-14",
        "chatcmpl-EahhoXeIIdUZY6Yb0Ccw8zIzYJuea",
    );
}

#[tokio::test]
async fn given_the_recorded_length_reply_when_called_then_the_response_is_the_cut_text_with_output_exhaustion()
 {
    let called = call_recorded(LENGTH, dialect(), &hello()).await;
    assert_eq!(texts(&called), ["Beneath the endless"]);
    assert_eq!(
        called.response().ending(),
        &ResponseEnding::output_exhaustion()
    );
    assert_eq!(called.response().usage(), Some(&usage(15, 5, 0)));
    assert_report(
        &called,
        LENGTH,
        "gpt-4.1-nano-2025-04-14",
        "chatcmpl-Eahhr3AAEYXtXnDIF8R3XISutDu1E",
    );
}

#[tokio::test]
async fn given_the_recorded_tool_reply_when_called_then_the_response_holds_the_call_with_the_tool_use_ending()
 {
    let weather = tool(
        "get_weather",
        "Get the weather of a city",
        vec![parameter("city", "The city", true, ParameterKind::String)],
    );
    let request = request_with(
        None,
        vec![user("What is the weather in Paris?")],
        vec![weather],
        OutputMode::Plain,
    );
    let called = call_recorded(TOOL, dialect(), &request).await;
    let calls = calls_of(called.response());
    assert_eq!(calls.len(), 1);
    let call = calls.first().expect("a call");
    assert_eq!(
        call.external_id().map(|id| id.as_str()),
        Some("call_ZJ4D7BK3ZGpJHd4vnCNAj22l")
    );
    assert_eq!(call.name().as_str(), "get_weather");
    assert_eq!(arguments_of(call), json!({"city": "Paris"}));
    assert_eq!(called.response().ending(), &ResponseEnding::tool_use());
    assert_eq!(called.response().usage(), Some(&usage(55, 30, 0)));
    assert!(texts(&called).is_empty());
    assert_report(
        &called,
        TOOL,
        "gpt-4.1-nano-2025-04-14",
        "chatcmpl-EahhprXaOW0Aw5te7JSXvI7bOTOOf",
    );
}

fn reasoning_dialect() -> ChatCompletionsDialect {
    let settings = ClientSettings::new(
        ModelName::new("gpt-5.4-mini").expect("model"),
        ReasoningLevel::Low,
        CacheRequirement::NotRequired,
        MaxOutputTokens::new(512).expect("tokens"),
    );
    ChatCompletionsDialect::new(settings, options()).expect("dialect")
}

#[tokio::test]
async fn given_the_recorded_reasoning_reply_when_called_at_low_then_the_request_asks_low_and_the_response_counts_the_reasoning_tokens_without_thinking_text()
 {
    let request = request_of(vec![user("What is 17 * 23?")]);
    let called = call_recorded(REASONING, reasoning_dialect(), &request).await;
    let sent: Value =
        serde_json::from_slice(&called.server.requests()[0].body).expect("a json body");
    assert_eq!(sent["reasoning_effort"], json!("low"));
    assert_eq!(sent["model"], json!("gpt-5.4-mini"));
    assert_eq!(texts(&called), ["391"]);
    assert_eq!(called.response().usage(), Some(&usage(20, 21, 11)));
    assert!(thinking_of(called.response()).is_empty());
    assert_report(
        &called,
        REASONING,
        "gpt-5.4-mini-2026-03-17",
        "chatcmpl-EahhsI8q2ReMFjmYEmShr56ieusVg",
    );
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
    request_with(None, vec![user("Do something harmful")], Vec::new(), output)
}

#[tokio::test]
async fn given_the_recorded_refusal_when_called_with_a_plain_output_then_the_response_is_a_refusal_holding_the_refusal_text()
 {
    let called = call_recorded(REFUSAL, dialect(), &hello()).await;
    assert_eq!(called.response().ending(), &ResponseEnding::refusal());
    assert_eq!(
        texts(&called),
        ["I'm sorry, but I can't assist with that request."]
    );
    assert_eq!(called.response().usage(), Some(&usage(50, 12, 0)));
    assert_report(
        &called,
        REFUSAL,
        "gpt-4.1-mini-2025-04-14",
        "chatcmpl-EahhsmJWZ9eY35X8XPZlMklA1CZKR",
    );
}

#[tokio::test]
async fn given_the_recorded_refusal_when_called_with_the_recorded_structured_output_then_the_call_is_content_blocked_and_the_sink_still_saw_the_refusal_text()
 {
    let called = call_recorded(REFUSAL, dialect(), &refusal_request()).await;
    assert_eq!(called.error(), &ClientError::ContentBlocked);
    assert_eq!(
        appended_text(&called.deltas),
        "I'm sorry, but I can't assist with that request."
    );
    let report = called.outcome.report();
    assert_eq!(report.request_id(), Some(REFUSAL.request_id()));
    assert_eq!(report.usage(), Some(&usage(50, 12, 0)));
}
