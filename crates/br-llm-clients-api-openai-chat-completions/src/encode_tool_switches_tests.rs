use br_llm_messages::{ContentBlock, PlainBlock, RuntimeToolCall, Turn};
use serde_json::{Value, json};

use crate::test_support::{dialect_for, encoded_body, messages_of, options, request_of};
use crate::test_thread::{
    answer, call, call_block, calling, in_progress, result_of, results, text, texts, thinking, user,
};
use crate::{
    ChatCompletionsOptions, ReasoningEffort, ReasoningLevel, ReasoningRule, ReasoningTable,
    ToolOnlyContent,
};

fn encoded(options: ChatCompletionsOptions, turns: Vec<Turn>) -> Vec<Value> {
    messages_of(
        &dialect_for(ReasoningLevel::Off, options),
        &request_of(turns),
    )
}

fn answered(
    calls: &[(RuntimeToolCall, Vec<PlainBlock>, bool)],
    blocks: Vec<ContentBlock>,
) -> Vec<Turn> {
    let mut blocks = blocks;
    blocks.extend(calls.iter().map(|(call, _, _)| call_block(call)));
    let answers = calls
        .iter()
        .map(|(call, content, is_error)| result_of(call, content.clone(), *is_error))
        .collect();
    vec![
        user("go"),
        in_progress(vec![calling(blocks), results(answers)]),
    ]
}

fn results_loop() -> Vec<Turn> {
    answered(
        &[
            (
                call("call_1", "weather", json!({})),
                texts(&["sunny"]),
                false,
            ),
            (
                call("call_2", "time", json!({})),
                texts(&["late", "very"]),
                true,
            ),
        ],
        Vec::new(),
    )
}

fn empty_result_loop() -> Vec<Turn> {
    answered(
        &[
            (call("call_1", "noop", json!({})), Vec::new(), false),
            (call("call_2", "one", json!({})), texts(&["x"]), false),
            (call("call_3", "two", json!({})), texts(&["x", "y"]), false),
        ],
        Vec::new(),
    )
}

fn tool_only_loop() -> Vec<Turn> {
    answered(
        &[(
            call("call_1", "weather", json!({})),
            texts(&["sunny"]),
            false,
        )],
        Vec::new(),
    )
}

#[test]
fn given_tool_message_name_on_when_encoded_then_each_tool_message_holds_the_tool_name_of_its_result()
 {
    let messages = encoded(options().with_tool_message_name(true), results_loop());
    assert_eq!(
        messages[2],
        json!({"role": "tool", "tool_call_id": "call_1", "name": "weather", "content": "sunny"})
    );
    assert_eq!(messages[3]["name"], json!("time"));
    assert_eq!(messages[3]["tool_call_id"], json!("call_2"));
}

#[test]
fn given_tool_message_name_off_when_encoded_then_no_tool_message_holds_a_name() {
    for configured in [options(), options().with_tool_message_name(false)] {
        let messages = encoded(configured, results_loop());
        for message in &messages[2..] {
            assert!(message.get("name").is_none(), "{message}");
        }
    }
}

#[test]
fn given_an_error_result_when_encoded_with_the_name_on_then_the_error_flag_is_still_not_carried() {
    let messages = encoded(options().with_tool_message_name(true), results_loop());
    assert_eq!(
        messages[3],
        json!({
            "role": "tool",
            "tool_call_id": "call_2",
            "name": "time",
            "content": [{"type": "text", "text": "late"}, {"type": "text", "text": "very"}]
        })
    );
}

#[test]
fn given_an_empty_tool_result_text_when_a_result_has_no_text_then_it_is_written_in_place_of_the_empty_string()
 {
    let messages = encoded(
        options().with_empty_tool_result_text("(no output)"),
        empty_result_loop(),
    );
    assert_eq!(messages[2]["content"], json!("(no output)"));
}

#[test]
fn given_an_empty_tool_result_text_when_a_result_has_texts_then_they_are_unchanged() {
    let messages = encoded(
        options().with_empty_tool_result_text("(no output)"),
        empty_result_loop(),
    );
    assert_eq!(messages[3]["content"], json!("x"));
    assert_eq!(
        messages[4]["content"],
        json!([{"type": "text", "text": "x"}, {"type": "text", "text": "y"}])
    );
}

#[test]
fn given_no_empty_tool_result_text_when_a_result_has_no_text_then_the_content_is_the_empty_string()
{
    let messages = encoded(options(), empty_result_loop());
    assert_eq!(messages[2]["content"], json!(""));
}

#[test]
fn given_empty_text_when_a_message_has_tool_calls_and_no_text_then_content_is_the_empty_string() {
    let messages = encoded(
        options().with_tool_only_content(ToolOnlyContent::EmptyText),
        tool_only_loop(),
    );
    assert_eq!(messages[1]["content"], json!(""));
    assert!(messages[1]["tool_calls"].is_array());
}

#[test]
fn given_omitted_when_a_message_has_tool_calls_and_no_text_then_content_is_not_written() {
    for configured in [
        options(),
        options().with_tool_only_content(ToolOnlyContent::Omitted),
    ] {
        let messages = encoded(configured, tool_only_loop());
        assert!(messages[1].get("content").is_none(), "{}", messages[1]);
    }
}

#[test]
fn given_empty_text_when_a_message_has_text_and_tool_calls_then_the_text_is_kept() {
    let turns = answered(
        &[(
            call("call_1", "weather", json!({})),
            texts(&["sunny"]),
            false,
        )],
        vec![text("checking")],
    );
    let messages = encoded(
        options().with_tool_only_content(ToolOnlyContent::EmptyText),
        turns,
    );
    assert_eq!(messages[1]["content"], json!("checking"));
}

#[test]
fn given_empty_text_when_a_message_has_no_text_and_no_tool_call_then_it_is_still_omitted() {
    let turns = vec![user("go"), in_progress(vec![answer(vec![thinking("hmm")])])];
    let messages = encoded(
        options().with_tool_only_content(ToolOnlyContent::EmptyText),
        turns,
    );
    assert_eq!(messages, [json!({"role": "user", "content": "go"})]);
}

#[test]
fn given_every_switch_of_a_thinking_host_set_together_when_encoded_then_each_one_shows_in_the_body()
{
    let table = ReasoningTable::new(
        ReasoningRule::Send(ReasoningEffort::None),
        ReasoningRule::Send(ReasoningEffort::Low),
        ReasoningRule::Send(ReasoningEffort::Medium),
        ReasoningRule::Omit,
        ReasoningRule::Omit,
        ReasoningRule::Omit,
    );
    let configured = ChatCompletionsOptions::new(table)
        .with_omit_effort_above_off(true)
        .with_reasoning_carry_back_field("reasoning_content")
        .with_tool_message_name(true)
        .with_tool_only_content(ToolOnlyContent::EmptyText);
    let turns = answered(
        &[(
            call("call_1", "weather", json!({})),
            texts(&["sunny"]),
            false,
        )],
        vec![thinking("check first")],
    );
    let body = encoded_body(
        &dialect_for(ReasoningLevel::High, configured),
        &request_of(turns),
    );
    assert!(body.get("reasoning_effort").is_none());
    assert_eq!(
        body["messages"][1],
        json!({
            "role": "assistant",
            "content": "",
            "reasoning_content": "check first",
            "tool_calls": [{"id": "call_1", "type": "function", "function": {"name": "weather", "arguments": "{}"}}]
        })
    );
    assert_eq!(
        body["messages"][2],
        json!({"role": "tool", "tool_call_id": "call_1", "name": "weather", "content": "sunny"})
    );
}
