use std::time::Duration;

use br_llm_messages::{
    ContentBlock, ExternalId, ModelName, ModelResponse, OpaqueJson, ResponseEnding,
    ResponseMetadata, RuntimeToolCall, StructuredBlock, ThinkingBlock, ToolName,
};
use serde_json::{Value, json};

use crate::test_support::usage;
use crate::thread_fixtures::{
    block_id, content_image, content_text, entry_id, provider_result_blocks,
};
use crate::{CallOutcome, CallReport, ClientError};

fn report() -> CallReport {
    CallReport::new(Duration::from_millis(900))
}

fn keys(value: &Value) -> Vec<&str> {
    value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect()
}

fn rich_response() -> ModelResponse {
    let arguments =
        OpaqueJson::parse(r#"{"a":1.5,"b":[1,2,{"c":null}],"big":18446744073709551615}"#).unwrap();
    let call = RuntimeToolCall::new(block_id(), ToolName::new("search").unwrap(), arguments)
        .unwrap()
        .with_external_id(ExternalId::new("call_1").unwrap());
    let mut blocks = vec![
        ContentBlock::Thinking(ThinkingBlock::new(block_id(), "hmm").unwrap()),
        content_text("hello"),
        content_image(br_llm_messages::MediaType::Png),
        ContentBlock::StructuredContent(
            StructuredBlock::new(
                block_id(),
                OpaqueJson::parse(r#"{"k":[true,0.25]}"#).unwrap(),
            )
            .unwrap(),
        ),
    ];
    blocks.extend(provider_result_blocks(br_llm_messages::MediaType::Gif));
    blocks.push(ContentBlock::RuntimeToolCall(call));
    let metadata = ResponseMetadata::new()
        .with_usage(
            usage(10, 5)
                .with_cache_read_input_tokens(4)
                .with_thinking_tokens(2),
        )
        .with_model(ModelName::new("served-model").unwrap())
        .with_provider_response_id(ExternalId::new("resp_1").unwrap());
    ModelResponse::new(entry_id(), blocks, ResponseEnding::tool_use(), metadata).unwrap()
}

pub(crate) fn rich_success() -> CallOutcome {
    let response = rich_response();
    let report = report()
        .with_usage(response.usage().unwrap().clone())
        .with_model(ModelName::new("served-model").unwrap())
        .with_request_id("req-1")
        .with_time_to_first_event(Duration::from_millis(120));
    CallOutcome::success(report, response).unwrap()
}

fn failure() -> CallOutcome {
    CallOutcome::failure(report(), ClientError::Transport)
}

#[test]
fn given_a_success_when_stored_then_the_keys_are_schema_report_and_a_result_with_status_and_response()
 {
    let json = serde_json::to_value(rich_success()).unwrap();
    assert_eq!(keys(&json), ["report", "result", "schema"]);
    assert_eq!(json["schema"], json!("br-llm-clients-core/2"));
    assert_eq!(keys(&json["result"]), ["response", "status"]);
    assert_eq!(json["result"]["status"], json!("success"));
    let stored_response = serde_json::to_value(rich_response()).unwrap();
    assert_eq!(keys(&json["result"]["response"]), keys(&stored_response));
}

#[test]
fn given_a_failure_when_stored_then_the_result_holds_status_and_error() {
    let json = serde_json::to_value(failure()).unwrap();
    assert_eq!(
        json,
        json!({
            "schema": "br-llm-clients-core/2",
            "report": { "total_duration": { "secs": 0, "nanos": 900_000_000 } },
            "result": { "status": "failure", "error": { "type": "transport" } }
        })
    );
}

#[test]
fn given_a_full_report_when_stored_then_absent_items_are_omitted_and_present_ones_have_their_spelling()
 {
    let full = report()
        .with_usage(usage(10, 5).with_thinking_tokens(2))
        .with_model(ModelName::new("m").unwrap())
        .with_request_id("req-1")
        .with_time_to_first_event(Duration::from_millis(120));
    assert_eq!(
        serde_json::to_value(&full).unwrap(),
        json!({
            "usage": { "input_tokens": 10, "output_tokens": 5, "thinking_tokens": 2 },
            "model": "m",
            "request_id": "req-1",
            "time_to_first_event": { "secs": 0, "nanos": 120_000_000 },
            "total_duration": { "secs": 0, "nanos": 900_000_000 }
        })
    );
    assert_eq!(
        serde_json::to_value(report()).unwrap(),
        json!({ "total_duration": { "secs": 0, "nanos": 900_000_000 } })
    );
}

#[test]
fn given_every_usage_count_when_stored_then_the_five_keys_are_spelled_as_the_text_says() {
    let all = usage(1, 2)
        .with_cache_read_input_tokens(3)
        .with_cache_creation_input_tokens(4)
        .with_thinking_tokens(5);
    let json = serde_json::to_value(report().with_usage(all)).unwrap();
    assert_eq!(
        keys(&json["usage"]),
        [
            "cache_creation_input_tokens",
            "cache_read_input_tokens",
            "input_tokens",
            "output_tokens",
            "thinking_tokens"
        ]
    );
}

#[test]
fn given_a_success_with_every_kind_of_block_when_round_tripped_then_identical() {
    let outcome = rich_success();
    let text = serde_json::to_string(&outcome).unwrap();
    assert_eq!(serde_json::from_str::<CallOutcome>(&text).unwrap(), outcome);
    let value = serde_json::to_value(&outcome).unwrap();
    assert_eq!(
        serde_json::from_value::<CallOutcome>(value).unwrap(),
        outcome
    );
}

#[test]
fn given_a_failure_with_the_last_usage_before_it_when_round_tripped_then_identical() {
    let outcome = CallOutcome::failure(
        report().with_usage(usage(7, 2)),
        ClientError::RateLimited {
            retry_after: Some(Duration::from_secs(2)),
        },
    );
    let json = serde_json::to_value(&outcome).unwrap();
    assert_eq!(
        serde_json::from_value::<CallOutcome>(json).unwrap(),
        outcome
    );
}

#[test]
fn given_a_duration_when_stored_then_it_is_an_object_with_secs_and_nanos() {
    let json =
        serde_json::to_value(report().with_time_to_first_event(Duration::new(3, 5))).unwrap();
    assert_eq!(
        json["time_to_first_event"],
        json!({ "secs": 3, "nanos": 5 })
    );
}

#[test]
fn given_a_stored_outcome_when_the_response_ending_is_stored_then_it_keeps_the_messages_spelling() {
    let json = serde_json::to_value(rich_success()).unwrap();
    assert_eq!(
        json["result"]["response"]["ending"],
        json!({ "type": "tool_use" })
    );
}
