use br_llm_clients_core::{ClientError, ReasoningLevel};
use br_llm_messages::{ContentBlock, ModelName, ReplayPayload, ResponseEnding, TurnEntry, Usage};
use serde_json::{Value, json};

use crate::test_history::{plain_text, reply, result_of, results, say};
use crate::test_recorded::{
    answer_request, recorded_event, reply_of, run_plain, run_recording, weather_request,
};
use crate::test_recordings::{INCOMPLETE, REASONING_TOOL, REFUSAL, TEXT};
use crate::test_requests::{dialect, encoded, request};
use crate::test_run::texts;

fn usage_with(input: u64, output: u64, thinking: u64) -> Usage {
    Usage::of_input_tokens(input)
        .with_cache_read_input_tokens(0)
        .with_cache_creation_input_tokens(0)
        .with_output_tokens(output)
        .with_thinking_tokens(thinking)
}

fn model_of(name: &str) -> ModelName {
    ModelName::new(name).expect("model")
}

fn joined(step: &br_llm_messages::ModelResponse) -> String {
    texts(step).concat()
}

#[tokio::test]
async fn given_the_recorded_text_reply_when_run_then_the_reply_report_and_ids_are_those_of_the_stream()
 {
    let called = run_plain(&TEXT).await;
    let report = called.outcome.report().clone();
    let step = reply_of(called);
    assert_eq!(*step.ending(), ResponseEnding::finished());
    assert_eq!(joined(&step), "Hello! How are you today?");
    assert_eq!(step.usage(), Some(&usage_with(13, 8, 0)));
    assert_eq!(step.model(), Some(&model_of("gpt-4.1-nano-2025-04-14")));
    let created = recorded_event(&TEXT, "response.created", 0);
    assert_eq!(
        step.provider_response_id().map(|id| id.as_str()),
        created["response"]["id"].as_str()
    );
    assert_eq!(report.request_id(), Some(TEXT.request_id));
    assert_eq!(report.usage(), Some(&usage_with(13, 8, 0)));
    assert_eq!(report.model(), Some(&model_of("gpt-4.1-nano-2025-04-14")));
}

#[tokio::test]
async fn given_the_recorded_cut_reply_when_run_then_the_reply_holds_the_partial_text_with_the_ending_output_exhaustion()
 {
    let step = reply_of(run_plain(&INCOMPLETE).await);
    assert_eq!(*step.ending(), ResponseEnding::output_exhaustion());
    assert_eq!(
        joined(&step),
        "Beneath the boundless canvas, azure wide,  \nWhere whispers of the"
    );
    assert_eq!(step.usage(), Some(&usage_with(15, 16, 0)));
}

#[tokio::test]
async fn given_the_recorded_refusal_when_run_as_plain_then_the_reply_holds_the_refusal_text_with_the_ending_refusal()
 {
    let step = reply_of(run_plain(&REFUSAL).await);
    assert_eq!(*step.ending(), ResponseEnding::refusal());
    assert_eq!(
        joined(&step),
        "I'm sorry, but I can't assist with that request."
    );
}

#[tokio::test]
async fn given_the_recorded_refusal_when_run_as_structured_then_the_call_fails_with_content_blocked_and_keeps_the_report()
 {
    let called = run_recording(&REFUSAL, ReasoningLevel::Off, &answer_request()).await;
    let report = called.outcome.report().clone();
    let (_, result) = called.outcome.into_parts();
    assert_eq!(result.expect_err("a failure"), ClientError::ContentBlocked);
    assert_eq!(report.request_id(), Some(REFUSAL.request_id));
    assert_eq!(report.usage(), Some(&usage_with(44, 13, 0)));
    assert_eq!(report.model(), Some(&model_of("gpt-4.1-mini-2025-04-14")));
}

#[tokio::test]
async fn given_the_recorded_reasoning_and_tool_reply_when_run_then_the_reply_holds_the_thinking_block_and_the_call_and_uses_a_tool()
 {
    let called = run_recording(&REASONING_TOOL, ReasoningLevel::Low, &weather_request()).await;
    let report = called.outcome.report().clone();
    let step = reply_of(called);
    assert_eq!(*step.ending(), ResponseEnding::tool_use());
    assert_eq!(step.usage(), Some(&usage_with(54, 30, 10)));
    assert_eq!(report.request_id(), Some(REASONING_TOOL.request_id));
    let [
        ContentBlock::Thinking(thinking),
        ContentBlock::RuntimeToolCall(call),
    ] = step.blocks()
    else {
        panic!("expected a thinking block then a tool call");
    };
    assert_eq!(thinking.text(), "");
    let done = recorded_event(&REASONING_TOOL, "response.output_item.done", 0)["item"].clone();
    let material = thinking.replay().expect("replay material");
    assert_eq!(
        material.format().as_str(),
        "br-llm-clients-api-openai-responses/reasoning"
    );
    let ReplayPayload::Text { value, .. } = material.payload() else {
        panic!("a text payload");
    };
    let envelope: Value = serde_json::from_str(value).expect("the envelope is JSON");
    assert_eq!(
        envelope,
        json!({ "id": done["id"], "encrypted_content": done["encrypted_content"] })
    );
    assert_eq!(
        call.external_id().map(|id| id.as_str()),
        Some("call_ncwwZsUdlv8xT4aac97RgMXa")
    );
    assert_eq!(call.name().as_str(), "get_weather");
    assert_eq!(
        call.arguments().to_json_string().expect("json"),
        "{\"city\":\"Paris\"}"
    );
}

#[tokio::test]
async fn given_the_recorded_reasoning_reply_when_sent_back_in_the_same_turn_then_the_reasoning_item_carries_the_real_id_and_ciphertext()
 {
    let called = run_recording(&REASONING_TOOL, ReasoningLevel::Low, &weather_request()).await;
    let step = reply_of(called);
    let call = crate::test_run::calls(&step).pop().expect("a call");
    let next = request(vec![
        say("What is the weather in Paris?"),
        reply(vec![
            TurnEntry::ModelResponse(step),
            results(vec![result_of(&call, vec![plain_text("15 C")], false)]),
        ]),
    ]);
    let encoded = encoded(&dialect(ReasoningLevel::Low), &next).expect("encoded");
    let body: Value = serde_json::from_slice(encoded.body()).expect("json");
    let done = recorded_event(&REASONING_TOOL, "response.output_item.done", 0)["item"].clone();
    let added = recorded_event(&REASONING_TOOL, "response.output_item.added", 0)["item"].clone();
    assert_eq!(
        body["input"][1],
        json!({
            "type": "reasoning",
            "id": done["id"],
            "summary": [],
            "encrypted_content": done["encrypted_content"]
        })
    );
    assert_ne!(
        body["input"][1]["encrypted_content"],
        added["encrypted_content"]
    );
    assert_eq!(
        body["input"][2],
        json!({
            "type": "function_call",
            "call_id": "call_ncwwZsUdlv8xT4aac97RgMXa",
            "name": "get_weather",
            "arguments": "{\"city\":\"Paris\"}"
        })
    );
    assert_eq!(
        body["input"][3],
        json!({
            "type": "function_call_output",
            "call_id": "call_ncwwZsUdlv8xT4aac97RgMXa",
            "output": "15 C"
        })
    );
    assert_eq!(body["input"].as_array().map(Vec::len), Some(4));
}
