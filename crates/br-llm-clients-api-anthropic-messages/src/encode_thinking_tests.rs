use serde_json::json;

use crate::test_fixtures::*;

fn conversation_with_tool_loop() -> Vec<br_llm_messages::WireMessage> {
    vec![
        user("first question"),
        assistant(vec![
            thinking("old reasoning", Some("old-signature")),
            redacted("old-redacted"),
            br_llm_messages::AssistantBlock::Text {
                text: text("old answer"),
            },
        ]),
        user("second question"),
        assistant(vec![
            thinking("fresh reasoning", Some("fresh-signature")),
            redacted("fresh-redacted"),
            tool_call("t1", "lookup", json!({})),
        ]),
        user_blocks(vec![tool_result("t1", "lookup", &["r"], false)]),
    ]
}

#[test]
fn given_thinking_of_the_turn_in_progress_when_encoded_then_blocks_keep_their_place_and_signature()
{
    let body = body_of(&dialect(), &request(conversation_with_tool_loop()));
    assert_eq!(
        body["messages"][3]["content"],
        json!([
            {"type": "thinking", "thinking": "fresh reasoning", "signature": "fresh-signature"},
            {"type": "redacted_thinking", "data": "fresh-redacted"},
            {"type": "tool_use", "id": "t1", "name": "lookup", "input": {}},
        ])
    );
}

#[test]
fn given_thinking_of_an_earlier_turn_when_encoded_then_it_is_not_sent() {
    let body = body_of(&dialect(), &request(conversation_with_tool_loop()));
    assert_eq!(
        body["messages"][1]["content"],
        json!([{"type": "text", "text": "old answer"}])
    );
    let sent = body.to_string();
    assert!(!sent.contains("old reasoning"));
    assert!(!sent.contains("old-signature"));
    assert!(!sent.contains("old-redacted"));
}

#[test]
fn given_an_earlier_thinking_without_signature_when_encoded_then_it_is_dropped_without_refusal() {
    let request = request(vec![
        user("a"),
        assistant(vec![
            thinking("unsigned", None),
            br_llm_messages::AssistantBlock::Text { text: text("b") },
        ]),
        user("c"),
    ]);
    let body = body_of(&dialect(), &request);
    assert_eq!(
        body["messages"][1]["content"],
        json!([{"type": "text", "text": "b"}])
    );
}

#[test]
fn given_an_empty_thinking_text_with_a_signature_in_the_turn_in_progress_when_encoded_then_it_is_sent_empty()
 {
    let request = request(vec![
        user("go"),
        assistant(vec![
            thinking("", Some("sig")),
            tool_call("t1", "lookup", json!({})),
        ]),
        user_blocks(vec![tool_result("t1", "lookup", &["r"], false)]),
    ]);
    let body = body_of(&dialect(), &request);
    assert_eq!(
        body["messages"][1]["content"][0],
        json!({"type": "thinking", "thinking": "", "signature": "sig"})
    );
}

#[test]
fn given_two_assistant_messages_in_one_turn_when_encoded_then_both_keep_their_thinking() {
    let request = request(vec![
        user("go"),
        assistant(vec![
            thinking("one", Some("s1")),
            tool_call("t1", "lookup", json!({})),
        ]),
        user_blocks(vec![tool_result("t1", "lookup", &["r"], false)]),
        assistant(vec![
            thinking("two", Some("s2")),
            tool_call("t2", "lookup", json!({})),
        ]),
        user_blocks(vec![tool_result("t2", "lookup", &["r"], false)]),
    ]);
    let body = body_of(&dialect(), &request);
    assert_eq!(body["messages"][1]["content"][0]["signature"], json!("s1"));
    assert_eq!(body["messages"][3]["content"][0]["signature"], json!("s2"));
}

#[test]
fn given_a_relay_after_a_tool_result_when_encoded_then_the_turn_in_progress_is_unchanged() {
    let request = request(vec![
        user("go"),
        assistant(vec![
            thinking("one", Some("s1")),
            tool_call("t1", "lookup", json!({})),
        ]),
        user_blocks(vec![tool_result("t1", "lookup", &["r"], false)]),
        relay(&["news"]),
    ]);
    let body = body_of(&dialect(), &request);
    assert_eq!(body["messages"][1]["content"][0]["type"], json!("thinking"));
}

#[test]
fn given_a_text_user_message_after_a_turn_when_encoded_then_that_turn_becomes_earlier() {
    let request = request(vec![
        user("go"),
        assistant(vec![
            thinking("one", Some("s1")),
            br_llm_messages::AssistantBlock::Text { text: text("done") },
        ]),
        user("again"),
    ]);
    let body = body_of(&dialect(), &request);
    assert_eq!(
        body["messages"][1]["content"],
        json!([{"type": "text", "text": "done"}])
    );
}
