use br_llm_messages::{
    AssistantBlock, RedactedThinking, Signature, Text, Thinking, WireMessage, WireUserBlock,
};
use serde_json::{Value, json};

use crate::test_support::{
    assistant, assistant_call, assistant_text, dialect_for, encoded_body, options, request_of,
    tool_result, user, user_text,
};
use crate::{ChatCompletionsOptions, ReasoningLevel};

fn carrying() -> ChatCompletionsOptions {
    options().with_reasoning_carry_back_field("reasoning_content")
}

fn messages_of(options: ChatCompletionsOptions, request_messages: Vec<WireMessage>) -> Vec<Value> {
    let request = request_of(request_messages);
    encoded_body(&dialect_for(ReasoningLevel::Off, options), &request)["messages"]
        .as_array()
        .cloned()
        .expect("messages")
}

fn unsigned(text: &str) -> AssistantBlock {
    AssistantBlock::Thinking(Thinking {
        text: text.to_owned(),
        signature: None,
    })
}

fn signed(text: &str) -> AssistantBlock {
    AssistantBlock::Thinking(Thinking {
        text: text.to_owned(),
        signature: Some(Signature::new("sig==").expect("signature")),
    })
}

fn redacted() -> AssistantBlock {
    AssistantBlock::RedactedThinking(RedactedThinking::new("opaque").expect("redacted"))
}

fn result(id: &str) -> WireMessage {
    user(vec![tool_result(id, "weather", &["sunny"], false)])
}

fn call(id: &str) -> AssistantBlock {
    assistant_call(id, "weather", json!({}))
}

#[test]
fn given_a_turn_in_progress_when_encoded_then_the_member_sits_beside_role_content_and_tool_calls() {
    let messages = messages_of(
        carrying(),
        vec![
            user_text("go"),
            assistant(vec![
                unsigned("I should look"),
                assistant_text("checking"),
                call("c1"),
            ]),
            result("c1"),
        ],
    );
    assert_eq!(
        messages[1],
        json!({
            "role": "assistant",
            "content": "checking",
            "reasoning_content": "I should look",
            "tool_calls": [{"id": "c1", "type": "function", "function": {"name": "weather", "arguments": "{}"}}]
        })
    );
}

#[test]
fn given_an_earlier_turn_when_encoded_then_only_the_turn_in_progress_gets_the_member() {
    let messages = messages_of(
        carrying(),
        vec![
            user_text("first"),
            assistant(vec![unsigned("old thought"), assistant_text("old answer")]),
            user_text("second"),
            assistant(vec![unsigned("new thought"), call("c1")]),
            result("c1"),
        ],
    );
    assert!(messages[1].get("reasoning_content").is_none());
    assert_eq!(messages[3]["reasoning_content"], json!("new thought"));
}

#[test]
fn given_several_unsigned_blocks_when_encoded_then_their_texts_are_joined_in_order_with_nothing_between()
 {
    let messages = messages_of(
        carrying(),
        vec![
            user_text("go"),
            assistant(vec![
                unsigned("ab"),
                assistant_text("mid"),
                unsigned("cd"),
                call("c1"),
            ]),
            result("c1"),
        ],
    );
    assert_eq!(messages[1]["reasoning_content"], json!("abcd"));
}

#[test]
fn given_signed_and_redacted_blocks_when_encoded_then_they_add_nothing_to_the_string() {
    let messages = messages_of(
        carrying(),
        vec![
            user_text("go"),
            assistant(vec![
                signed("secret"),
                unsigned("kept"),
                redacted(),
                call("c1"),
            ]),
            result("c1"),
        ],
    );
    assert_eq!(messages[1]["reasoning_content"], json!("kept"));
}

#[test]
fn given_only_signed_or_redacted_blocks_when_encoded_then_the_member_is_not_written() {
    let messages = messages_of(
        carrying(),
        vec![
            user_text("go"),
            assistant(vec![signed("secret"), redacted(), call("c1")]),
            result("c1"),
        ],
    );
    assert!(messages[1].get("reasoning_content").is_none());
}

#[test]
fn given_an_unsigned_block_with_no_text_when_encoded_then_the_member_is_not_written() {
    let messages = messages_of(
        carrying(),
        vec![
            user_text("go"),
            assistant(vec![unsigned(""), call("c1")]),
            result("c1"),
        ],
    );
    assert!(messages[1].get("reasoning_content").is_none());
}

#[test]
fn given_the_option_unset_when_encoded_then_no_thinking_is_written() {
    let messages = messages_of(
        options(),
        vec![
            user_text("go"),
            assistant(vec![unsigned("thought"), assistant_text("hi"), call("c1")]),
            result("c1"),
        ],
    );
    assert!(
        !Value::Array(messages.clone())
            .to_string()
            .contains("thought")
    );
    assert!(messages[1].get("reasoning_content").is_none());
}

#[test]
fn given_a_message_left_empty_when_encoded_then_its_thinking_is_dropped_and_moved_nowhere() {
    let messages = messages_of(
        carrying(),
        vec![
            user_text("go"),
            assistant(vec![unsigned("lost thought")]),
            assistant(vec![call("c1")]),
            result("c1"),
        ],
    );
    assert_eq!(messages.len(), 3);
    assert!(
        !Value::Array(messages.clone())
            .to_string()
            .contains("lost thought")
    );
    assert!(messages[1].get("reasoning_content").is_none());
}

#[test]
fn given_several_messages_in_the_turn_when_encoded_then_each_keeps_its_own_thinking() {
    let messages = messages_of(
        carrying(),
        vec![
            user_text("go"),
            assistant(vec![unsigned("one"), call("c1")]),
            result("c1"),
            assistant(vec![unsigned("two"), call("c2")]),
            result("c2"),
        ],
    );
    assert_eq!(messages[1]["reasoning_content"], json!("one"));
    assert_eq!(messages[3]["reasoning_content"], json!("two"));
}

#[test]
fn given_a_user_message_with_a_result_and_text_when_encoded_then_it_does_not_end_the_turn() {
    let both = user(vec![
        tool_result("c1", "weather", &["sunny"], false),
        WireUserBlock::Text {
            text: Text::new("also").expect("text"),
        },
    ]);
    let messages = messages_of(
        carrying(),
        vec![
            user_text("go"),
            assistant(vec![unsigned("one"), call("c1")]),
            both,
            assistant(vec![unsigned("two"), call("c2")]),
            result("c2"),
        ],
    );
    assert_eq!(messages[1]["reasoning_content"], json!("one"));
    assert_eq!(messages[4]["reasoning_content"], json!("two"));
}

#[test]
fn given_a_relay_when_encoded_then_it_does_not_end_the_turn() {
    let relay = WireMessage::Relay {
        content: vec![Text::new("note").expect("text")],
    };
    let messages = messages_of(
        carrying(),
        vec![
            user_text("go"),
            assistant(vec![unsigned("one"), call("c1")]),
            result("c1"),
            relay,
        ],
    );
    assert_eq!(messages[1]["reasoning_content"], json!("one"));
}

#[test]
fn given_no_user_message_without_a_result_when_encoded_then_every_assistant_message_is_of_the_turn()
{
    let messages = messages_of(
        carrying(),
        vec![
            result("c0"),
            assistant(vec![unsigned("one"), call("c1")]),
            result("c1"),
        ],
    );
    assert_eq!(messages[1]["reasoning_content"], json!("one"));
}

#[test]
fn given_a_different_name_and_any_reasoning_level_when_encoded_then_the_member_is_written() {
    let configured = options().with_reasoning_carry_back_field("reasoning");
    let request = request_of(vec![
        user_text("go"),
        assistant(vec![unsigned("one"), call("c1")]),
        result("c1"),
    ]);
    for level in [ReasoningLevel::Off, ReasoningLevel::High] {
        let body = encoded_body(&dialect_for(level, configured.clone()), &request);
        assert_eq!(body["messages"][1]["reasoning"], json!("one"));
    }
}

#[test]
fn given_the_option_set_when_encoded_twice_then_the_bytes_are_equal() {
    let request = request_of(vec![
        user_text("go"),
        assistant(vec![unsigned("one"), call("c1")]),
        result("c1"),
    ]);
    let dialect = dialect_for(ReasoningLevel::Off, carrying());
    let first = br_llm_clients_common_call::Dialect::encode(&dialect, &request).expect("first");
    let second = br_llm_clients_common_call::Dialect::encode(&dialect, &request).expect("second");
    assert_eq!(first.body(), second.body());
}
