use br_llm_messages::{ContentBlock, RuntimeToolCall, Turn, TurnEntry};
use serde_json::{Value, json};

use crate::test_support::{dialect_for, encode, messages_of, options, request_of};
use crate::test_thread::{
    answer, call, call_block, calling, earlier, in_progress, redacted, result_of, results, signed,
    text, texts, thinking, user,
};
use crate::{ChatCompletionsOptions, ReasoningLevel};

fn carrying() -> ChatCompletionsOptions {
    options().with_reasoning_carry_back_field("reasoning_content")
}

fn encoded(options: ChatCompletionsOptions, turns: Vec<Turn>) -> Vec<Value> {
    messages_of(
        &dialect_for(ReasoningLevel::Off, options),
        &request_of(turns),
    )
}

fn weather(id: &str) -> RuntimeToolCall {
    call(id, "weather", json!({}))
}

fn step(call: &RuntimeToolCall, mut blocks: Vec<ContentBlock>) -> Vec<TurnEntry> {
    blocks.push(call_block(call));
    vec![
        calling(blocks),
        results(vec![result_of(call, texts(&["sunny"]), false)]),
    ]
}

fn turn_of(blocks: Vec<ContentBlock>) -> Vec<Turn> {
    vec![user("go"), in_progress(step(&weather("c1"), blocks))]
}

#[test]
fn given_a_turn_in_progress_when_encoded_then_the_member_sits_beside_role_content_and_tool_calls() {
    let messages = encoded(
        carrying(),
        turn_of(vec![thinking("I should look"), text("checking")]),
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
fn given_an_earlier_turn_with_thinking_when_encoded_then_only_the_turn_in_progress_gets_the_member()
{
    let turns = vec![
        user("first"),
        earlier(vec![answer(vec![
            thinking("old thought"),
            text("old answer"),
        ])]),
        user("second"),
        in_progress(step(&weather("c1"), vec![thinking("new thought")])),
    ];
    let messages = encoded(carrying(), turns);
    assert_eq!(
        messages[1],
        json!({"role": "assistant", "content": "old answer"})
    );
    assert_eq!(messages[3]["reasoning_content"], json!("new thought"));
    assert!(!Value::Array(messages).to_string().contains("old thought"));
}

#[test]
fn given_several_plain_thinking_blocks_when_encoded_then_their_texts_are_joined_in_order_with_nothing_between()
 {
    let messages = encoded(
        carrying(),
        turn_of(vec![thinking("ab"), text("mid"), thinking("cd")]),
    );
    assert_eq!(messages[1]["reasoning_content"], json!("abcd"));
}

#[test]
fn given_thinking_with_replay_material_and_redacted_thinking_when_encoded_then_they_add_nothing() {
    let messages = encoded(
        carrying(),
        turn_of(vec![signed("secret"), thinking("kept"), redacted()]),
    );
    assert_eq!(messages[1]["reasoning_content"], json!("kept"));
    assert!(!Value::Array(messages).to_string().contains("secret"));
}

#[test]
fn given_only_thinking_with_replay_material_or_redacted_thinking_when_encoded_then_the_member_is_not_written()
 {
    let messages = encoded(carrying(), turn_of(vec![signed("secret"), redacted()]));
    assert!(messages[1].get("reasoning_content").is_none());
}

#[test]
fn given_the_option_unset_when_encoded_then_no_thinking_is_written() {
    let messages = encoded(options(), turn_of(vec![thinking("thought"), text("hi")]));
    assert!(
        !Value::Array(messages.clone())
            .to_string()
            .contains("thought")
    );
    assert_eq!(messages[1].as_object().expect("object").len(), 3);
}

#[test]
fn given_a_response_left_empty_beside_a_kept_one_when_encoded_then_their_merged_message_carries_the_thinking_of_both()
 {
    let lookup = weather("c1");
    let turns = vec![
        user("go"),
        in_progress(vec![
            answer(vec![thinking("first ")]),
            calling(vec![thinking("second"), call_block(&lookup)]),
            results(vec![result_of(&lookup, texts(&["sunny"]), false)]),
        ]),
    ];
    let messages = encoded(carrying(), turns);
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[1]["reasoning_content"], json!("first second"));
}

#[test]
fn given_a_message_left_with_no_part_when_encoded_then_its_thinking_is_dropped_and_moved_nowhere() {
    let turns = vec![
        user("go"),
        in_progress(vec![answer(vec![thinking("lost thought")])]),
    ];
    let messages = encoded(carrying(), turns);
    assert_eq!(messages, [json!({"role": "user", "content": "go"})]);
}

#[test]
fn given_several_messages_in_the_turn_when_encoded_then_each_keeps_its_own_thinking() {
    let mut entries = step(&weather("c1"), vec![thinking("one")]);
    entries.extend(step(&weather("c2"), vec![thinking("two")]));
    let messages = encoded(carrying(), vec![user("go"), in_progress(entries)]);
    assert_eq!(messages[1]["reasoning_content"], json!("one"));
    assert_eq!(messages[3]["reasoning_content"], json!("two"));
}

#[test]
fn given_a_different_name_and_any_reasoning_level_when_encoded_then_the_member_is_written() {
    let configured = options().with_reasoning_carry_back_field("reasoning");
    let request = request_of(turn_of(vec![thinking("one")]));
    for level in [ReasoningLevel::Off, ReasoningLevel::High] {
        let messages = messages_of(&dialect_for(level, configured.clone()), &request);
        assert_eq!(messages[1]["reasoning"], json!("one"));
        assert!(messages[1].get("reasoning_content").is_none());
    }
}

#[test]
fn given_the_option_set_when_encoded_twice_then_the_bytes_are_equal() {
    let request = request_of(turn_of(vec![thinking("one")]));
    let dialect = dialect_for(ReasoningLevel::Off, carrying());
    let first = encode(&dialect, &request).expect("first");
    let second = encode(&dialect, &request).expect("second");
    assert_eq!(first.body(), second.body());
}
