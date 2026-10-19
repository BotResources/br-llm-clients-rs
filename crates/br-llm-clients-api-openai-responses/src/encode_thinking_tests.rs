use br_llm_messages::AssistantBlock;
use serde_json::json;

use crate::test_requests::{
    assistant, call_block, envelope_text, input_of, result_block, text, thinking, user, user_text,
};

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
