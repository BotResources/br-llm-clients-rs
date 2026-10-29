use br_llm_clients_core::ClientError;
use br_llm_messages::{ContentBlock, MediaType, ProviderToolCall, StructuredBlock};
use serde_json::{Value, json};

use crate::test_history::{
    call, earlier, fresh_block, image, json as opaque, notice, redacted, reply, response,
    result_of, results, say, text, thinking,
};
use crate::test_requests::{expected_function_call, input_of, refusal_of};

fn assistant_text(text: &str) -> Value {
    json!({ "type": "message", "role": "assistant", "content": text })
}

fn user_texts(texts: &[&str]) -> Value {
    let parts: Vec<Value> = texts
        .iter()
        .map(|text| json!({ "type": "input_text", "text": text }))
        .collect();
    json!({ "type": "message", "role": "user", "content": parts })
}

fn is_invalid(error: &ClientError, words: &str) -> bool {
    matches!(error, ClientError::InvalidMessages { reason } if reason.contains(words))
}

const LAST: &str = "the last message is an assistant message";

#[test]
fn given_an_assistant_text_when_encoded_then_it_is_a_message_with_a_string_content_and_no_id() {
    let items = input_of(vec![
        say("hi"),
        earlier(vec![response(vec![text("hello there")])]),
        say("thanks"),
    ]);
    assert_eq!(items[1], assistant_text("hello there"));
    assert!(items[1].get("id").is_none());
}

#[test]
fn given_a_structured_block_when_encoded_then_the_content_is_its_compact_json_text() {
    let structured =
        StructuredBlock::new(fresh_block(), opaque("{ \"a\" : [1, 2] }")).expect("structured");
    let items = input_of(vec![
        say("hi"),
        earlier(vec![response(vec![ContentBlock::StructuredContent(
            structured,
        )])]),
        say("more"),
    ]);
    assert_eq!(items[1], assistant_text("{\"a\":[1,2]}"));
}

#[test]
fn given_a_tool_call_when_encoded_then_arguments_is_compact_json_text_and_there_is_no_id() {
    let found = call(Some("call_9"), "search", "{ \"q\" : \"x\" }");
    let items = input_of(vec![
        say("hi"),
        earlier(vec![
            response(vec![ContentBlock::RuntimeToolCall(found.clone())]),
            results(vec![result_of(&found, Vec::new(), false)]),
        ]),
    ]);
    assert_eq!(
        items[1],
        expected_function_call("call_9", "search", "{\"q\":\"x\"}")
    );
    assert!(items[1].get("id").is_none());
}

#[test]
fn given_a_tool_call_without_an_external_id_when_encoded_then_the_call_and_its_output_carry_the_wire_id_of_the_core()
 {
    let found = call(None, "search", "{}");
    let items = input_of(vec![
        say("hi"),
        earlier(vec![
            response(vec![ContentBlock::RuntimeToolCall(found.clone())]),
            results(vec![result_of(&found, Vec::new(), false)]),
        ]),
    ]);
    let wire = format!("call_{}", found.id().as_uuid().simple());
    assert_eq!(items[1]["call_id"], json!(wire));
    assert_eq!(items[2]["call_id"], json!(wire));
}

#[test]
fn given_parts_of_several_kinds_when_encoded_then_there_is_one_item_per_part_in_order() {
    let found = call(Some("c1"), "search", "{}");
    let items = input_of(vec![
        say("hi"),
        reply(vec![
            response(vec![
                text("first"),
                ContentBlock::RuntimeToolCall(found.clone()),
                text("second"),
            ]),
            results(vec![result_of(&found, Vec::new(), false)]),
        ]),
    ]);
    assert_eq!(
        items[1..4],
        [
            assistant_text("first"),
            expected_function_call("c1", "search", "{}"),
            assistant_text("second"),
        ]
    );
}

#[test]
fn given_a_redacted_thinking_block_when_encoded_then_it_is_not_sent() {
    let items = input_of(vec![
        say("hi"),
        reply(vec![
            response(vec![redacted("opaque"), text("answer")]),
            notice("go on"),
        ]),
    ]);
    assert_eq!(
        items,
        vec![
            user_texts(&["hi"]),
            assistant_text("answer"),
            user_texts(&["<notice>go on</notice>"]),
        ]
    );
}

#[test]
fn given_an_assistant_message_whose_parts_all_drop_out_when_encoded_then_it_gives_no_item_and_its_neighbours_merge()
 {
    let items = input_of(vec![
        say("hi"),
        reply(vec![
            response(vec![redacted("opaque"), thinking("no replay", None)]),
            notice("go on"),
        ]),
    ]);
    assert_eq!(items, vec![user_texts(&["hi", "<notice>go on</notice>"])]);
}

#[test]
fn given_two_assistant_messages_that_drop_out_when_encoded_then_the_three_user_messages_become_one()
{
    let items = input_of(vec![
        say("one"),
        reply(vec![
            response(vec![thinking("a", None)]),
            notice("two"),
            response(vec![redacted("b")]),
            notice("three"),
        ]),
    ]);
    assert_eq!(
        items,
        vec![user_texts(&[
            "one",
            "<notice>two</notice>",
            "<notice>three</notice>"
        ])]
    );
}

#[test]
fn given_an_assistant_message_that_drops_out_before_a_tool_result_message_when_encoded_then_the_parts_follow_in_order()
 {
    let found = call(Some("c1"), "search", "{}");
    let items = input_of(vec![
        say("hi"),
        reply(vec![
            response(vec![ContentBlock::RuntimeToolCall(found.clone())]),
            results(vec![result_of(&found, Vec::new(), false)]),
            response(vec![thinking("x", None)]),
            notice("after"),
        ]),
    ]);
    let kinds: Vec<&str> = items
        .iter()
        .map(|item| item["type"].as_str().expect("type"))
        .collect();
    assert_eq!(
        kinds,
        [
            "message",
            "function_call",
            "function_call_output",
            "message"
        ]
    );
    assert_eq!(items[3], user_texts(&["<notice>after</notice>"]));
}

#[test]
fn given_a_history_call_to_an_undeclared_tool_when_encoded_then_it_is_not_refused() {
    let found = call(Some("c1"), "never_declared", "{}");
    let items = input_of(vec![
        say("hi"),
        earlier(vec![
            response(vec![ContentBlock::RuntimeToolCall(found.clone())]),
            results(vec![result_of(&found, Vec::new(), false)]),
        ]),
    ]);
    assert_eq!(items[1]["name"], json!("never_declared"));
}

#[test]
fn given_a_text_block_with_replay_material_of_the_reply_turn_when_encoded_then_it_is_sent_as_a_text()
 {
    let block = br_llm_messages::TextBlock::new(fresh_block(), "carried")
        .expect("text")
        .with_replay(crate::test_history::envelope_replay("msg_1", "E"));
    let items = input_of(vec![
        say("hi"),
        reply(vec![
            response(vec![ContentBlock::Text(block)]),
            notice("go"),
        ]),
    ]);
    assert_eq!(items[1], assistant_text("carried"));
}

#[test]
fn given_a_last_assistant_message_when_encoded_then_the_messages_are_invalid() {
    let error = refusal_of(vec![say("hi"), reply(vec![response(vec![text("x")])])]);
    assert!(is_invalid(&error, LAST), "{error:?}");
}

#[test]
fn given_a_last_assistant_message_of_only_a_call_when_encoded_then_the_messages_are_invalid() {
    let found = call(Some("c1"), "search", "{}");
    let error = refusal_of(vec![
        say("hi"),
        earlier(vec![
            response(vec![ContentBlock::RuntimeToolCall(found.clone())]),
            results(vec![result_of(&found, Vec::new(), false)]),
        ]),
        reply(vec![response(vec![text("and then")])]),
    ]);
    assert!(is_invalid(&error, LAST), "{error:?}");
}

#[test]
fn given_a_last_assistant_message_whose_parts_all_drop_out_when_encoded_then_the_messages_are_invalid()
 {
    let error = refusal_of(vec![
        say("hi"),
        reply(vec![response(vec![thinking("t", None), redacted("r")])]),
    ]);
    assert!(is_invalid(&error, LAST), "{error:?}");
}

#[test]
fn given_an_image_in_an_assistant_message_when_encoded_then_the_messages_are_invalid() {
    let error = refusal_of(vec![
        say("hi"),
        earlier(vec![response(vec![ContentBlock::Image(image(
            MediaType::Png,
            "QUJD",
        ))])]),
        say("again"),
    ]);
    assert!(is_invalid(&error, "holds an image"), "{error:?}");
}

#[test]
fn given_a_provider_tool_block_of_the_reply_turn_when_encoded_then_the_messages_are_invalid() {
    let block = ContentBlock::ProviderToolCall(ProviderToolCall::new(fresh_block()));
    let error = refusal_of(vec![
        say("hi"),
        reply(vec![response(vec![block, text("x")]), notice("go")]),
    ]);
    assert!(
        is_invalid(&error, "a block that this dialect cannot send"),
        "{error:?}"
    );
}
