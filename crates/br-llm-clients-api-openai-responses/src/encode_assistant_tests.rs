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

#[test]
fn given_the_text_of_a_thinking_block_when_encoded_then_it_is_never_sent() {
    let signature = envelope_text("rs_1", "ENC");
    let messages = vec![
        user_text("go"),
        assistant(vec![
            thinking("SECRET THINKING TEXT", Some(&signature)),
            call_block("c", "search", json!({})),
        ]),
        user(vec![result_block("c", "search", &["r"], false)]),
    ];
    let body = serde_json::to_string(&input_of(messages)).expect("json");
    assert!(!body.contains("SECRET THINKING TEXT"));
}

#[test]
fn given_a_thinking_block_of_an_earlier_turn_when_encoded_then_it_is_dropped_without_error() {
    let signature = envelope_text("rs_old", "OLD");
    let messages = vec![
        user_text("first question"),
        assistant(vec![
            thinking("old", Some(&signature)),
            AssistantBlock::Text {
                text: text("first answer"),
            },
        ]),
        user_text("second question"),
    ];
    let items = input_of(messages);
    assert_eq!(items.len(), 3);
    assert!(items.iter().all(|item| item["type"] != json!("reasoning")));
}

#[test]
fn given_two_steps_in_the_same_turn_when_encoded_then_both_thinking_blocks_are_sent() {
    let first = envelope_text("rs_1", "E1");
    let second = envelope_text("rs_2", "E2");
    let messages = vec![
        user_text("go"),
        assistant(vec![
            thinking("a", Some(&first)),
            call_block("c1", "search", json!({})),
        ]),
        user(vec![result_block("c1", "search", &["r1"], false)]),
        assistant(vec![
            thinking("b", Some(&second)),
            call_block("c2", "search", json!({})),
        ]),
        user(vec![result_block("c2", "search", &["r2"], false)]),
    ];
    let items = input_of(messages);
    let ids: Vec<&str> = items
        .iter()
        .filter(|item| item["type"] == json!("reasoning"))
        .map(|item| item["id"].as_str().expect("id"))
        .collect();
    assert_eq!(ids, ["rs_1", "rs_2"]);
}

#[test]
fn given_a_user_message_with_text_and_a_result_when_encoded_then_it_still_counts_as_in_the_turn() {
    let signature = envelope_text("rs_1", "E1");
    let messages = vec![
        user_text("go"),
        assistant(vec![
            thinking("a", Some(&signature)),
            call_block("c1", "search", json!({})),
        ]),
        user(vec![
            result_block("c1", "search", &["r1"], false),
            crate::test_requests::user_text_block("and this note"),
        ]),
    ];
    let items = input_of(messages);
    assert!(items.iter().any(|item| item["type"] == json!("reasoning")));
}

#[test]
fn given_a_relay_after_the_turn_when_encoded_then_the_turn_in_progress_stays_the_same() {
    let signature = envelope_text("rs_1", "E1");
    let messages = vec![
        user_text("go"),
        assistant(vec![
            thinking("a", Some(&signature)),
            call_block("c1", "search", json!({})),
        ]),
        user(vec![result_block("c1", "search", &["r1"], false)]),
        assistant(vec![
            thinking("b", Some(&envelope_text("rs_2", "E2"))),
            AssistantBlock::Text {
                text: text("answer"),
            },
        ]),
        crate::test_requests::relay("someone wrote"),
    ];
    let items = input_of(messages);
    let ids: Vec<&str> = items
        .iter()
        .filter(|item| item["type"] == json!("reasoning"))
        .map(|item| item["id"].as_str().expect("id"))
        .collect();
    assert_eq!(ids, ["rs_1", "rs_2"]);
}

#[test]
fn given_a_user_message_after_the_thinking_when_encoded_then_the_earlier_turn_is_dropped() {
    let signature = envelope_text("rs_1", "E1");
    let messages = vec![
        user_text("go"),
        assistant(vec![
            thinking("a", Some(&signature)),
            AssistantBlock::Text {
                text: text("answer"),
            },
        ]),
        user_text("and then?"),
        assistant(vec![
            thinking("b", Some(&envelope_text("rs_2", "E2"))),
            call_block("c", "search", json!({})),
        ]),
        user(vec![result_block("c", "search", &["r"], false)]),
    ];
    let items = input_of(messages);
    let ids: Vec<&str> = items
        .iter()
        .filter(|item| item["type"] == json!("reasoning"))
        .map(|item| item["id"].as_str().expect("id"))
        .collect();
    assert_eq!(ids, ["rs_2"]);
}

#[test]
fn given_signatures_that_are_not_reasoning_envelopes_when_encoded_then_the_blocks_are_dropped_silently()
 {
    let bad_signatures = [
        "opaque-anthropic-signature".to_owned(),
        "[]".to_owned(),
        "\"text\"".to_owned(),
        "{}".to_owned(),
        "{\"id\":\"rs_1\"}".to_owned(),
        "{\"encrypted_content\":\"E\"}".to_owned(),
        "{\"id\":\"\",\"encrypted_content\":\"E\"}".to_owned(),
        "{\"id\":\"rs_1\",\"encrypted_content\":\"\"}".to_owned(),
        "{\"id\":1,\"encrypted_content\":\"E\"}".to_owned(),
        "{\"id\":\"rs_1\",\"encrypted_content\":null}".to_owned(),
        "{\"id\":\"rs_1\",\"encrypted_content\":\"E\",\"extra\":1}".to_owned(),
    ];
    for signature in bad_signatures {
        let messages = vec![
            user_text("go"),
            assistant(vec![
                thinking("a", Some(&signature)),
                call_block("c", "search", json!({})),
            ]),
            user(vec![result_block("c", "search", &["r"], false)]),
        ];
        let items = input_of(messages);
        assert!(
            items.iter().all(|item| item["type"] != json!("reasoning")),
            "{signature}"
        );
        assert_eq!(items.len(), 3, "{signature}");
    }
}

#[test]
fn given_a_thinking_block_without_signature_when_encoded_then_it_is_dropped_without_error() {
    let messages = vec![
        user_text("go"),
        assistant(vec![
            thinking("no signature", None),
            call_block("c", "search", json!({})),
        ]),
        user(vec![result_block("c", "search", &["r"], false)]),
    ];
    assert_eq!(input_of(messages).len(), 3);
}

#[test]
fn given_a_history_without_a_plain_user_message_when_encoded_then_no_thinking_is_sent() {
    let signature = envelope_text("rs_1", "E1");
    let messages = vec![
        user(vec![result_block("c0", "search", &["r0"], false)]),
        assistant(vec![
            thinking("a", Some(&signature)),
            call_block("c1", "search", json!({})),
        ]),
        user(vec![result_block("c1", "search", &["r1"], false)]),
    ];
    let items = input_of(messages);
    assert!(items.iter().all(|item| item["type"] != json!("reasoning")));
}
