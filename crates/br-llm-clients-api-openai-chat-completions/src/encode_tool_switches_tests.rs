use serde_json::{Value, json};

use crate::test_support::{
    assistant, assistant_call, assistant_text, dialect_for, encoded_body, options, request_of,
    tool_result, user, user_text,
};
use crate::{ChatCompletionsOptions, ReasoningLevel, ToolOnlyContent};
use br_llm_messages::{AssistantBlock, Thinking};

fn messages_of(
    options: ChatCompletionsOptions,
    request_messages: Vec<br_llm_messages::WireMessage>,
) -> Vec<Value> {
    let request = request_of(request_messages);
    encoded_body(&dialect_for(ReasoningLevel::Off, options), &request)["messages"]
        .as_array()
        .cloned()
        .expect("messages")
}

fn results_loop() -> Vec<br_llm_messages::WireMessage> {
    vec![
        user_text("go"),
        assistant(vec![
            assistant_call("call_1", "weather", json!({})),
            assistant_call("call_2", "time", json!({})),
        ]),
        user(vec![
            tool_result("call_1", "weather", &["sunny"], false),
            tool_result("call_2", "time", &["late", "very"], true),
        ]),
    ]
}

#[test]
fn given_tool_message_name_on_when_encoded_then_each_tool_message_holds_the_tool_name_of_its_result()
 {
    let messages = messages_of(options().with_tool_message_name(true), results_loop());
    assert_eq!(
        messages[2],
        json!({"role": "tool", "tool_call_id": "call_1", "name": "weather", "content": "sunny"})
    );
    assert_eq!(messages[3]["name"], json!("time"));
    assert_eq!(messages[3]["tool_call_id"], json!("call_2"));
}

#[test]
fn given_tool_message_name_off_when_encoded_then_no_tool_message_holds_a_name() {
    let messages = messages_of(options().with_tool_message_name(false), results_loop());
    for message in &messages[2..] {
        assert!(message.get("name").is_none(), "{message}");
    }
}

#[test]
fn given_an_error_result_when_encoded_with_the_name_on_then_the_error_flag_is_still_not_carried() {
    let messages = messages_of(options().with_tool_message_name(true), results_loop());
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

fn empty_result_loop() -> Vec<br_llm_messages::WireMessage> {
    vec![
        user_text("go"),
        assistant(vec![assistant_call("call_1", "noop", json!({}))]),
        user(vec![
            tool_result("call_1", "noop", &[], false),
            tool_result("call_2", "one", &["x"], false),
            tool_result("call_3", "two", &["x", "y"], false),
        ]),
    ]
}

#[test]
fn given_an_empty_tool_result_text_when_a_result_has_no_text_then_it_is_written_in_place_of_the_empty_string()
 {
    let messages = messages_of(
        options().with_empty_tool_result_text("(no output)"),
        empty_result_loop(),
    );
    assert_eq!(messages[2]["content"], json!("(no output)"));
}

#[test]
fn given_an_empty_tool_result_text_when_a_result_has_texts_then_they_are_unchanged() {
    let messages = messages_of(
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
    let messages = messages_of(options(), empty_result_loop());
    assert_eq!(messages[2]["content"], json!(""));
}

fn tool_only_loop() -> Vec<br_llm_messages::WireMessage> {
    vec![
        user_text("go"),
        assistant(vec![assistant_call("call_1", "weather", json!({}))]),
        user(vec![tool_result("call_1", "weather", &["sunny"], false)]),
    ]
}

#[test]
fn given_empty_text_when_a_message_has_tool_calls_and_no_text_then_content_is_the_empty_string() {
    let messages = messages_of(
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
        let messages = messages_of(configured, tool_only_loop());
        assert!(messages[1].get("content").is_none(), "{}", messages[1]);
    }
}

#[test]
fn given_empty_text_when_a_message_has_text_and_tool_calls_then_the_text_is_kept() {
    let messages = messages_of(
        options().with_tool_only_content(ToolOnlyContent::EmptyText),
        vec![
            user_text("go"),
            assistant(vec![
                assistant_text("checking"),
                assistant_call("call_1", "weather", json!({})),
            ]),
            user(vec![tool_result("call_1", "weather", &["sunny"], false)]),
        ],
    );
    assert_eq!(messages[1]["content"], json!("checking"));
}

#[test]
fn given_empty_text_when_a_message_has_no_text_and_no_tool_call_then_it_is_still_omitted() {
    let messages = messages_of(
        options().with_tool_only_content(ToolOnlyContent::EmptyText),
        vec![
            user_text("go"),
            assistant(vec![AssistantBlock::Thinking(Thinking {
                text: "hmm".to_owned(),
                signature: None,
            })]),
            user_text("again"),
        ],
    );
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[1]["role"], json!("user"));
}
