use br_llm_messages::{ContentBlock, ExternalId};
use serde_json::{Value, json};

use crate::test_support::{dialect, encode, messages_of, request_of};
use crate::test_thread::{
    answer, call, call_block, calling, earlier, in_progress, redacted, result_of, results, said,
    signed, structured, text, text_with_replay, texts, thinking, user,
};

fn between(blocks: Vec<ContentBlock>) -> Value {
    let request = request_of(vec![
        user("go"),
        earlier(vec![answer(blocks)]),
        user("again"),
    ]);
    messages_of(&dialect(), &request)[1].clone()
}

fn in_turn(blocks: Vec<ContentBlock>) -> Value {
    let lookup = call("c9", "lookup", json!({}));
    let mut blocks = blocks;
    blocks.push(call_block(&lookup));
    let answers = blocks
        .iter()
        .filter_map(|block| match block {
            ContentBlock::RuntimeToolCall(call) => Some(result_of(call, texts(&["x"]), false)),
            _ => None,
        })
        .collect();
    let request = request_of(vec![
        user("go"),
        in_progress(vec![calling(blocks), results(answers)]),
    ]);
    messages_of(&dialect(), &request)[1].clone()
}

#[test]
fn given_one_text_part_when_encoded_then_content_is_a_string_and_no_tool_calls_member_exists() {
    assert_eq!(
        between(vec![text("hello")]),
        json!({"role": "assistant", "content": "hello"})
    );
}

#[test]
fn given_several_text_parts_when_encoded_then_content_is_an_array_of_text_parts_in_order() {
    assert_eq!(
        between(vec![text("a"), text("b")])["content"],
        json!([{"type": "text", "text": "a"}, {"type": "text", "text": "b"}])
    );
}

#[test]
fn given_a_structured_block_when_encoded_then_its_part_holds_the_compact_json_text() {
    let value = json!({"answer": 42, "tags": ["a", "b"]});
    let message = between(vec![structured(value.clone())]);
    let written = message["content"].as_str().expect("string");
    assert!(!written.contains(' '));
    assert_eq!(serde_json::from_str::<Value>(written).expect("json"), value);
}

#[test]
fn given_text_and_structured_blocks_when_encoded_then_they_are_parts_in_order() {
    assert_eq!(
        between(vec![structured(json!([1])), text("then text")])["content"],
        json!([{"type": "text", "text": "[1]"}, {"type": "text", "text": "then text"}])
    );
}

#[test]
fn given_a_tool_call_when_encoded_then_it_has_the_wire_id_the_function_type_the_name_and_text_arguments()
 {
    let message = in_turn(vec![call_block(&call(
        "call_1",
        "lookup",
        json!({"q": "rust", "n": 3}),
    ))]);
    let written = &message["tool_calls"][0];
    assert_eq!(written["id"], json!("call_1"));
    assert_eq!(written["type"], json!("function"));
    assert_eq!(written["function"]["name"], json!("lookup"));
    let arguments = written["function"]["arguments"].as_str().expect("text");
    assert!(!arguments.contains(' '));
    assert_eq!(
        serde_json::from_str::<Value>(arguments).expect("json"),
        json!({"q": "rust", "n": 3})
    );
    assert_eq!(written.as_object().expect("object").len(), 3);
    assert_eq!(written["function"].as_object().expect("object").len(), 2);
}

#[test]
fn given_a_message_with_only_tool_calls_when_encoded_then_no_content_member_is_written() {
    let message = in_turn(Vec::new());
    let members = message.as_object().expect("object");
    assert!(!members.contains_key("content"));
    assert_eq!(members["tool_calls"].as_array().expect("array").len(), 1);
}

#[test]
fn given_text_between_tool_calls_when_encoded_then_the_text_is_content_and_the_calls_keep_their_order()
 {
    let message = in_turn(vec![
        call_block(&call("c1", "first", json!({}))),
        text("thinking aloud"),
        call_block(&call("c2", "second", json!({"x": 1}))),
    ]);
    assert_eq!(message["content"], json!("thinking aloud"));
    let names: Vec<&str> = message["tool_calls"]
        .as_array()
        .expect("array")
        .iter()
        .filter_map(|written| written["function"]["name"].as_str())
        .collect();
    assert_eq!(names, ["first", "second", "lookup"]);
}

#[test]
fn given_thinking_and_redacted_thinking_of_the_turn_when_encoded_then_both_are_dropped() {
    let message = in_turn(vec![
        thinking("hmm"),
        signed("sealed"),
        redacted(),
        text("answer"),
    ]);
    assert_eq!(message["content"], json!("answer"));
    assert_eq!(message.as_object().expect("object").len(), 3);
    assert!(!message.to_string().contains("hmm"));
    assert!(!message.to_string().contains("sealed"));
}

#[test]
fn given_a_text_block_with_replay_material_when_encoded_then_its_text_is_sent_without_it() {
    let message = in_turn(vec![text_with_replay("kept text")]);
    assert_eq!(message["content"], json!("kept text"));
    assert!(!message.to_string().contains("opaque"));
    assert!(!message.to_string().contains("item_1"));
}

#[test]
fn given_a_tool_call_with_replay_material_when_encoded_then_it_is_sent_without_it() {
    let with_replay = call("c1", "lookup", json!({}))
        .with_replay(crate::test_thread::material("test.call", "opaque"))
        .with_item_ref(br_llm_messages::ItemRef::new("item_7").expect("item ref"));
    let message = in_turn(vec![call_block(&with_replay)]);
    assert!(!message.to_string().contains("opaque"));
    assert!(!message.to_string().contains("item_7"));
    assert_eq!(message["tool_calls"][0]["id"], json!("c1"));
}

#[test]
fn given_a_call_with_no_external_id_when_encoded_then_the_wire_id_of_the_rendering_is_sent_on_both_sides()
 {
    let bare = br_llm_messages::RuntimeToolCall::new(
        crate::test_thread::block_id(),
        crate::test_support::tool_name("lookup"),
        br_llm_messages::OpaqueJson::parse("{}").expect("json"),
    )
    .expect("call");
    let request = request_of(vec![
        user("go"),
        earlier(vec![
            calling(vec![call_block(&bare)]),
            results(vec![result_of(&bare, texts(&["x"]), false)]),
        ]),
    ]);
    let messages = messages_of(&dialect(), &request);
    let id = messages[1]["tool_calls"][0]["id"].as_str().expect("id");
    assert!(id.starts_with("call_"));
    assert_eq!(messages[2]["tool_call_id"], json!(id));
}

#[test]
fn given_an_external_id_of_any_form_when_encoded_then_it_is_sent_unchanged_on_the_call_and_its_result()
 {
    let odd = call("toolu_01:Ab/c=", "lookup", json!({}));
    assert!(ExternalId::new("toolu_01:Ab/c=").is_ok());
    let request = request_of(vec![
        user("go"),
        earlier(vec![
            calling(vec![call_block(&odd)]),
            results(vec![result_of(&odd, texts(&["x"]), false)]),
        ]),
    ]);
    let messages = messages_of(&dialect(), &request);
    assert_eq!(messages[1]["tool_calls"][0]["id"], json!("toolu_01:Ab/c="));
    assert_eq!(messages[2]["tool_call_id"], json!("toolu_01:Ab/c="));
}

#[test]
fn given_a_tool_call_with_no_declared_tool_when_encoded_then_it_is_not_refused() {
    let undeclared = call("c1", "undeclared", json!({}));
    let request = request_of(vec![
        user("go"),
        earlier(vec![
            calling(vec![call_block(&undeclared)]),
            results(vec![result_of(&undeclared, texts(&["x"]), false)]),
        ]),
    ]);
    assert!(encode(&dialect(), &request).is_ok());
}

#[test]
fn given_an_earlier_answer_when_encoded_then_it_is_an_assistant_message_between_the_users() {
    let request = request_of(vec![user("go"), said("done"), user("again")]);
    let messages = messages_of(&dialect(), &request);
    assert_eq!(
        messages,
        [
            json!({"role": "user", "content": "go"}),
            json!({"role": "assistant", "content": "done"}),
            json!({"role": "user", "content": "again"}),
        ]
    );
}
