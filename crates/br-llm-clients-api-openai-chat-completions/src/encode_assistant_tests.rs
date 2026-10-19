use br_llm_clients_common_call::{Dialect, is_generated_id};
use br_llm_messages::{AssistantBlock, RedactedThinking, Signature, Thinking};
use serde_json::{Value, json};

use crate::test_support::{
    assistant, assistant_call, assistant_text, dialect, encoded_body, request_of, tool_result,
    user, user_text,
};

fn messages_with(content: Vec<AssistantBlock>) -> Value {
    let request = request_of(vec![
        user_text("go"),
        assistant(content),
        user_text("again"),
    ]);
    encoded_body(&dialect(), &request)["messages"].clone()
}

fn thinking() -> AssistantBlock {
    AssistantBlock::Thinking(Thinking {
        text: "let me think".to_owned(),
        signature: Some(Signature::new("sig==").expect("signature")),
    })
}

fn redacted() -> AssistantBlock {
    AssistantBlock::RedactedThinking(RedactedThinking::new("opaque").expect("redacted"))
}

#[test]
fn given_one_text_block_when_encoded_then_content_is_a_string_and_no_tool_calls_member_exists() {
    let messages = messages_with(vec![assistant_text("hello")]);
    assert_eq!(
        messages[1],
        json!({"role": "assistant", "content": "hello"})
    );
}

#[test]
fn given_several_text_blocks_when_encoded_then_content_is_an_array_of_text_parts_in_order() {
    let messages = messages_with(vec![assistant_text("a"), assistant_text("b")]);
    assert_eq!(
        messages[1]["content"],
        json!([{"type": "text", "text": "a"}, {"type": "text", "text": "b"}])
    );
}

#[test]
fn given_a_structured_block_when_encoded_then_its_part_holds_the_compact_json_text() {
    let value = json!({"answer": 42, "tags": ["a", "b"]});
    let messages = messages_with(vec![AssistantBlock::Structured {
        value: value.clone(),
    }]);
    let written = messages[1]["content"].as_str().expect("string");
    assert_eq!(written, value.to_string());
    assert!(!written.contains(' '));
    assert_eq!(serde_json::from_str::<Value>(written).expect("json"), value);
}

#[test]
fn given_text_and_structured_blocks_when_encoded_then_they_are_parts_in_block_order() {
    let messages = messages_with(vec![
        AssistantBlock::Structured { value: json!([1]) },
        assistant_text("then text"),
    ]);
    assert_eq!(
        messages[1]["content"],
        json!([{"type": "text", "text": "[1]"}, {"type": "text", "text": "then text"}])
    );
}

#[test]
fn given_a_tool_call_when_encoded_then_it_has_the_id_the_function_type_the_name_and_text_arguments()
{
    let messages = messages_with(vec![assistant_call(
        "call_1",
        "lookup",
        json!({"q": "rust", "n": 3}),
    )]);
    let call = &messages[1]["tool_calls"][0];
    assert_eq!(call["id"], json!("call_1"));
    assert_eq!(call["type"], json!("function"));
    assert_eq!(call["function"]["name"], json!("lookup"));
    let arguments = call["function"]["arguments"].as_str().expect("text");
    assert_eq!(arguments, json!({"q": "rust", "n": 3}).to_string());
    assert!(!arguments.contains(' '));
    assert_eq!(call.as_object().expect("object").len(), 3);
}

#[test]
fn given_a_message_with_only_tool_calls_when_encoded_then_no_content_member_is_written() {
    let messages = messages_with(vec![assistant_call("c1", "lookup", json!({}))]);
    let message = messages[1].as_object().expect("object");
    assert!(!message.contains_key("content"));
    assert_eq!(message["tool_calls"].as_array().expect("array").len(), 1);
}

#[test]
fn given_text_and_tool_calls_when_encoded_then_the_text_is_content_and_the_calls_keep_their_order()
{
    let messages = messages_with(vec![
        assistant_call("c1", "first", json!({})),
        assistant_text("thinking aloud"),
        assistant_call("c2", "second", json!({"x": 1})),
    ]);
    assert_eq!(messages[1]["content"], json!("thinking aloud"));
    let names: Vec<&str> = messages[1]["tool_calls"]
        .as_array()
        .expect("array")
        .iter()
        .filter_map(|call| call["function"]["name"].as_str())
        .collect();
    assert_eq!(names, ["first", "second"]);
}

#[test]
fn given_thinking_and_redacted_thinking_when_encoded_then_both_are_dropped_and_the_text_stays() {
    let messages = messages_with(vec![thinking(), redacted(), assistant_text("answer")]);
    assert_eq!(
        messages[1],
        json!({"role": "assistant", "content": "answer"})
    );
}

#[test]
fn given_a_message_holding_only_thinking_when_encoded_then_the_message_is_omitted() {
    let messages = messages_with(vec![thinking(), redacted()]);
    let roles: Vec<&str> = messages
        .as_array()
        .expect("array")
        .iter()
        .filter_map(|message| message["role"].as_str())
        .collect();
    assert_eq!(roles, ["user", "user"]);
}

#[test]
fn given_an_empty_assistant_message_when_encoded_then_the_message_is_omitted() {
    let messages = messages_with(Vec::new());
    assert_eq!(messages.as_array().expect("array").len(), 2);
}

#[test]
fn given_thinking_beside_a_tool_call_when_encoded_then_only_the_call_is_written() {
    let messages = messages_with(vec![thinking(), assistant_call("c1", "lookup", json!({}))]);
    assert_eq!(
        messages[1],
        json!({"role": "assistant", "tool_calls": [{
            "id": "c1", "type": "function",
            "function": {"name": "lookup", "arguments": "{}"}
        }]})
    );
}

#[test]
fn given_a_generated_id_when_encoded_then_it_is_sent_unchanged_on_the_call_and_its_result() {
    let generated = "brgen_0001";
    assert!(is_generated_id(generated));
    let request = request_of(vec![
        user_text("go"),
        assistant(vec![assistant_call(generated, "lookup", json!({}))]),
        user(vec![tool_result(generated, "lookup", &["x"], false)]),
    ]);
    let messages = encoded_body(&dialect(), &request)["messages"].clone();
    assert_eq!(messages[1]["tool_calls"][0]["id"], json!(generated));
    assert_eq!(messages[2]["tool_call_id"], json!(generated));
}

#[test]
fn given_a_tool_call_with_no_declared_tool_when_encoded_then_it_is_not_refused() {
    let request = request_of(vec![
        user_text("go"),
        assistant(vec![assistant_call("c1", "undeclared", json!({}))]),
        user(vec![tool_result("c1", "undeclared", &["x"], false)]),
    ]);
    assert!(dialect().encode(&request).is_ok());
}
