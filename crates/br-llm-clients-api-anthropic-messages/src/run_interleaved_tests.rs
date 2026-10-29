use br_llm_messages::{ModelResponse, ResponseEnding, TurnEntry};
use serde_json::json;

use crate::test_call::*;
use crate::test_fixtures::{body_of, dialect, request};
use crate::test_response::{thinking, tool_calls};
use crate::test_thread::{in_progress, result_of, results, signature, texts, user};

const INTERLEAVED: &str = r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_1","type":"message","role":"assistant","model":"claude-test-model","content":[],"stop_reason":null,"usage":{"input_tokens":10,"output_tokens":2}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"sig-first"}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: content_block_start
data: {"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"toolu_capital","name":"lookup_capital","input":{}}}

event: content_block_delta
data: {"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{\"country\": \"Japan\"}"}}

event: content_block_stop
data: {"type":"content_block_stop","index":1}

event: content_block_start
data: {"type":"content_block_start","index":2,"content_block":{"type":"thinking","thinking":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":2,"delta":{"type":"thinking_delta","thinking":"Now the clock."}}

event: content_block_delta
data: {"type":"content_block_delta","index":2,"delta":{"type":"signature_delta","signature":"sig-second"}}

event: content_block_stop
data: {"type":"content_block_stop","index":2}

event: content_block_start
data: {"type":"content_block_start","index":3,"content_block":{"type":"tool_use","id":"toolu_clock","name":"read_clock","input":{}}}

event: content_block_stop
data: {"type":"content_block_stop","index":3}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"tool_use","stop_sequence":null},"usage":{"output_tokens":40}}

event: message_stop
data: {"type":"message_stop"}

"#;

fn kinds(response: &ModelResponse) -> Vec<&'static str> {
    response
        .blocks()
        .iter()
        .map(|block| block.kind().as_str())
        .collect()
}

#[tokio::test]
async fn given_thinking_between_two_tool_calls_with_the_last_call_held_when_run_then_the_blocks_keep_the_stream_order()
 {
    let called = call_stream(&hello(), INTERLEAVED).await;
    let response = called.response();
    assert_eq!(response.ending(), &ResponseEnding::tool_use());
    assert_eq!(
        kinds(response),
        [
            "thinking",
            "runtime_tool_call",
            "thinking",
            "runtime_tool_call"
        ]
    );
    assert_eq!(
        thinking(response),
        [
            (String::new(), Some(signature("sig-first"))),
            ("Now the clock.".to_owned(), Some(signature("sig-second"))),
        ]
    );
    let calls = tool_calls(response);
    let calls = calls
        .iter()
        .map(|call| (call.id.as_deref(), call.name.as_str(), &call.arguments))
        .collect::<Vec<_>>();
    assert_eq!(
        calls,
        [
            (
                Some("toolu_capital"),
                "lookup_capital",
                &json!({"country": "Japan"})
            ),
            (Some("toolu_clock"), "read_clock", &json!({})),
        ]
    );
}

#[tokio::test]
async fn given_an_interleaved_reply_in_the_turn_in_progress_when_encoded_then_its_blocks_and_signatures_go_back_in_order()
 {
    let called = call_stream(&hello(), INTERLEAVED).await;
    let response = called.response().clone();
    let answers = response
        .runtime_calls()
        .map(|call| result_of(call, texts(&[call.name().as_str()]), false))
        .collect();
    let turns = vec![
        user("go"),
        in_progress(vec![TurnEntry::ModelResponse(response), results(answers)]),
    ];
    let body = body_of(&dialect(), &request(turns));
    assert_eq!(
        body["messages"][1],
        json!({"role": "assistant", "content": [
            {"type": "thinking", "thinking": "", "signature": "sig-first"},
            {"type": "tool_use", "id": "toolu_capital", "name": "lookup_capital", "input": {"country": "Japan"}},
            {"type": "thinking", "thinking": "Now the clock.", "signature": "sig-second"},
            {"type": "tool_use", "id": "toolu_clock", "name": "read_clock", "input": {}},
        ]})
    );
    assert_eq!(
        body["messages"][2],
        json!({"role": "user", "content": [
            {"type": "tool_result", "tool_use_id": "toolu_capital", "content": [{"type": "text", "text": "lookup_capital"}]},
            {"type": "tool_result", "tool_use_id": "toolu_clock", "content": [{"type": "text", "text": "read_clock"}]},
        ]})
    );
}
