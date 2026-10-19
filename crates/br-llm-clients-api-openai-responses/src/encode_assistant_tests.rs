use br_llm_messages::{AssistantBlock, RedactedThinking};
use serde_json::json;

use crate::test_requests::{
    assistant, call_block, envelope_text, expected_function_call, input_of, result_block, text,
    thinking, user, user_text,
};

fn with_assistant(blocks: Vec<AssistantBlock>) -> Vec<serde_json::Value> {
    input_of(vec![user_text("go"), assistant(blocks), user_text("next")])
}

#[test]
fn given_an_assistant_text_when_encoded_then_it_is_a_message_with_a_string_content_and_no_id() {
    let items = with_assistant(vec![AssistantBlock::Text { text: text("done") }]);
    assert_eq!(
        items[1],
        json!({ "type": "message", "role": "assistant", "content": "done" })
    );
}

#[test]
fn given_a_structured_block_when_encoded_then_the_content_is_its_compact_json_text() {
    let items = with_assistant(vec![AssistantBlock::Structured {
        value: json!({ "answer": [1, 2], "ok": true }),
    }]);
    assert_eq!(items[1]["role"], json!("assistant"));
    let content = items[1]["content"].as_str().expect("string content");
    assert!(!content.contains(' ') && !content.contains('\n'));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(content).expect("json"),
        json!({ "answer": [1, 2], "ok": true })
    );
}

#[test]
fn given_a_tool_call_when_encoded_then_arguments_is_compact_json_text_and_there_is_no_id() {
    let items = with_assistant(vec![call_block("call_9", "search", json!({ "q": "rust" }))]);
    assert_eq!(
        items[1],
        expected_function_call("call_9", "search", "{\"q\":\"rust\"}")
    );
    assert!(items[1].get("id").is_none());
}

#[test]
fn given_a_generated_tool_call_id_when_encoded_then_it_is_sent_as_it_is() {
    let items = with_assistant(vec![call_block("brgen_3", "search", json!({}))]);
    assert_eq!(items[1]["call_id"], json!("brgen_3"));
}

#[test]
fn given_blocks_of_several_kinds_when_encoded_then_there_is_one_item_per_block_in_order() {
    let items = with_assistant(vec![
        AssistantBlock::Text {
            text: text("let me look"),
        },
        call_block("a", "search", json!({})),
        call_block("b", "fetch", json!({ "n": 1 })),
    ]);
    let kinds: Vec<&str> = items[1..4]
        .iter()
        .map(|item| item["type"].as_str().expect("type"))
        .collect();
    assert_eq!(kinds, ["message", "function_call", "function_call"]);
    assert_eq!(items[3]["name"], json!("fetch"));
}

#[test]
fn given_a_redacted_thinking_block_when_encoded_then_it_is_not_sent() {
    let items = with_assistant(vec![
        AssistantBlock::RedactedThinking(RedactedThinking::new("opaque").expect("data")),
        AssistantBlock::Text { text: text("hi") },
    ]);
    assert_eq!(items.len(), 3);
    assert_eq!(items[1]["content"], json!("hi"));
}

#[test]
fn given_an_assistant_message_whose_blocks_all_drop_out_when_encoded_then_it_gives_no_item() {
    let items = with_assistant(vec![
        AssistantBlock::RedactedThinking(RedactedThinking::new("opaque").expect("data")),
        thinking("x", None),
    ]);
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["role"], json!("user"));
    assert_eq!(items[1]["role"], json!("user"));
}

#[test]
fn given_a_history_call_to_an_undeclared_tool_when_encoded_then_it_is_not_refused() {
    let items = with_assistant(vec![call_block("c", "never_declared", json!({}))]);
    assert_eq!(items[1]["name"], json!("never_declared"));
}

#[test]
fn given_a_thinking_block_of_the_turn_in_progress_when_encoded_then_a_reasoning_item_is_sent() {
    let signature = envelope_text("rs_1", "ENC");
    let messages = vec![
        user_text("go"),
        assistant(vec![
            thinking("shown to the caller only", Some(&signature)),
            call_block("c", "search", json!({})),
        ]),
        user(vec![result_block("c", "search", &["r"], false)]),
    ];
    let items = input_of(messages);
    assert_eq!(
        items[1],
        json!({ "type": "reasoning", "id": "rs_1", "summary": [], "encrypted_content": "ENC" })
    );
    assert_eq!(items[2]["type"], json!("function_call"));
}
