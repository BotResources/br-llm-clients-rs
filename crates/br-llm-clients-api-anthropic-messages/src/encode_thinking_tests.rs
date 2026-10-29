use br_llm_messages::Turn;
use serde_json::json;

use crate::test_fixtures::*;
use crate::test_thread::*;

fn conversation_with_tool_loop() -> Vec<Turn> {
    let call = call("t1", "lookup", json!({}));
    vec![
        user("first question"),
        earlier(vec![answer(vec![
            thinking("old reasoning", Some(signature("old-signature"))),
            redacted(redacted_data("old-redacted")),
            text("old answer"),
        ])]),
        user("second question"),
        in_progress(vec![
            calling(vec![
                thinking("fresh reasoning", Some(signature("fresh-signature"))),
                redacted(redacted_data("fresh-redacted")),
                call_block(&call),
            ]),
            results(vec![result_of(&call, texts(&["r"]), false)]),
        ]),
    ]
}

#[test]
fn given_thinking_of_the_turn_in_progress_when_encoded_then_blocks_keep_their_place_and_replay_material()
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
fn given_an_earlier_thinking_without_replay_material_when_encoded_then_it_is_dropped_without_refusal()
 {
    let request = request(vec![
        user("a"),
        earlier(vec![answer(vec![thinking("unsigned", None), text("b")])]),
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
    let call = call("t1", "lookup", json!({}));
    let request = request(vec![
        user("go"),
        in_progress(vec![
            calling(vec![
                thinking("", Some(signature("sig"))),
                call_block(&call),
            ]),
            results(vec![result_of(&call, texts(&["r"]), false)]),
        ]),
    ]);
    let body = body_of(&dialect(), &request);
    assert_eq!(
        body["messages"][1]["content"][0],
        json!({"type": "thinking", "thinking": "", "signature": "sig"})
    );
}

#[test]
fn given_two_responses_in_the_turn_in_progress_when_encoded_then_both_keep_their_thinking() {
    let first = call("t1", "lookup", json!({}));
    let second = call("t2", "lookup", json!({}));
    let request = request(vec![
        user("go"),
        in_progress(vec![
            calling(vec![
                thinking("one", Some(signature("s1"))),
                call_block(&first),
            ]),
            results(vec![result_of(&first, texts(&["r"]), false)]),
            calling(vec![
                thinking("two", Some(signature("s2"))),
                call_block(&second),
            ]),
            results(vec![result_of(&second, texts(&["r"]), false)]),
        ]),
    ]);
    let body = body_of(&dialect(), &request);
    assert_eq!(body["messages"][1]["content"][0]["signature"], json!("s1"));
    assert_eq!(body["messages"][3]["content"][0]["signature"], json!("s2"));
}

#[test]
fn given_the_replay_formats_when_named_then_they_are_the_formats_of_this_library() {
    assert_eq!(
        crate::replay_formats::SIGNATURE_FORMAT,
        "br-llm-clients-api-anthropic-messages/signature"
    );
    assert_eq!(
        crate::replay_formats::REDACTED_DATA_FORMAT,
        "br-llm-clients-api-anthropic-messages/redacted-data"
    );
}
