use br_llm_clients_common_call::Dialect;
use br_llm_clients_core::{ClientError, ReasoningLevel};
use br_llm_messages::{AssistantBlock, ModelId, StopReason, Usage, WireMessage};
use serde_json::{Value, json};

use crate::test_recorded::{
    answer_request, recorded_event, run_plain, run_recording, step_of, weather_request,
};
use crate::test_recordings::{INCOMPLETE, REASONING_TOOL, REFUSAL, TEXT};
use crate::test_requests::{assistant, dialect, request, result_block, user, user_text};

fn usage_with(input: u64, output: u64, thinking: u64) -> Usage {
    Usage {
        input_tokens: input,
        output_tokens: output,
        cache_read_tokens: Some(0),
        cache_write_tokens: Some(0),
        thinking_tokens: Some(thinking),
    }
}

fn model_of(name: &str) -> ModelId {
    ModelId::new(name).expect("model")
}

#[tokio::test]
async fn given_the_recorded_text_reply_when_run_then_the_step_report_and_request_id_are_those_of_the_stream()
 {
    let called = run_plain(&TEXT).await;
    let report = called.outcome.report().clone();
    let step = step_of(called);
    assert_eq!(*step.stop_reason(), StopReason::EndTurn);
    assert_eq!(
        step.text().map(|part| part.as_str()).collect::<String>(),
        "Hello! How are you today?"
    );
    assert_eq!(step.usage(), Some(&usage_with(13, 8, 0)));
    assert_eq!(step.model(), Some(&model_of("gpt-4.1-nano-2025-04-14")));
    assert_eq!(report.request_id(), Some(TEXT.request_id));
    assert_eq!(report.usage(), Some(&usage_with(13, 8, 0)));
    assert_eq!(report.model(), Some(&model_of("gpt-4.1-nano-2025-04-14")));
}

#[tokio::test]
async fn given_the_recorded_cut_reply_when_run_then_the_step_holds_the_partial_text_with_the_stop_max_tokens()
 {
    let step = step_of(run_plain(&INCOMPLETE).await);
    assert_eq!(*step.stop_reason(), StopReason::MaxTokens);
    assert_eq!(
        step.text().map(|part| part.as_str()).collect::<String>(),
        "Beneath the boundless canvas, azure wide,  \nWhere whispers of the"
    );
    assert_eq!(step.usage(), Some(&usage_with(15, 16, 0)));
}

#[tokio::test]
async fn given_the_recorded_refusal_when_run_as_plain_then_the_step_holds_the_refusal_text_with_the_stop_refusal()
 {
    let step = step_of(run_plain(&REFUSAL).await);
    assert_eq!(*step.stop_reason(), StopReason::Refusal);
    assert_eq!(
        step.text().map(|part| part.as_str()).collect::<String>(),
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
async fn given_the_recorded_reasoning_and_tool_reply_when_run_then_the_step_holds_the_thinking_block_and_the_call_and_awaits_the_result()
 {
    let called = run_recording(&REASONING_TOOL, ReasoningLevel::Low, &weather_request()).await;
    let report = called.outcome.report().clone();
    let step = step_of(called);
    assert_eq!(*step.stop_reason(), StopReason::AwaitingToolResults);
    assert_eq!(step.usage(), Some(&usage_with(54, 30, 10)));
    assert_eq!(report.request_id(), Some(REASONING_TOOL.request_id));
    let [
        AssistantBlock::Thinking(thinking),
        AssistantBlock::ToolCall(call),
    ] = step.content()
    else {
        panic!("expected a thinking block then a tool call");
    };
    assert_eq!(thinking.text, "");
    let done = recorded_event(&REASONING_TOOL, "response.output_item.done", 0)["item"].clone();
    let signature: Value =
        serde_json::from_str(thinking.signature.as_ref().expect("a signature").as_str())
            .expect("the envelope is JSON");
    assert_eq!(
        signature,
        json!({ "id": done["id"], "encrypted_content": done["encrypted_content"] })
    );
    assert_eq!(call.id.as_str(), "call_ncwwZsUdlv8xT4aac97RgMXa");
    assert_eq!(call.name.as_str(), "get_weather");
    assert_eq!(call.arguments, json!({ "city": "Paris" }));
}

#[tokio::test]
async fn given_the_step_of_the_recorded_reasoning_reply_when_sent_back_in_the_same_turn_then_the_reasoning_item_carries_the_real_id_and_ciphertext()
 {
    let called = run_recording(&REASONING_TOOL, ReasoningLevel::Low, &weather_request()).await;
    let step = step_of(called);
    let next = request(vec![
        user_text("What is the weather in Paris?"),
        assistant(step.content().to_vec()),
        user(vec![result_block(
            "call_ncwwZsUdlv8xT4aac97RgMXa",
            "get_weather",
            &["15 C"],
            false,
        )]),
    ]);
    let encoded = dialect(ReasoningLevel::Low).encode(&next).expect("encoded");
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
    assert_eq!(body["input"][3]["type"], "function_call_output");
    assert!(matches!(
        next.messages().last(),
        Some(WireMessage::User { .. })
    ));
}
