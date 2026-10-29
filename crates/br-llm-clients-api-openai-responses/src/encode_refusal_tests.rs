use br_llm_clients_core::ClientError;
use br_llm_messages::{ContentBlock, MediaType, ProviderToolCall};

use crate::test_history::{
    call, earlier, fresh_block, image, notice, redacted, reply, response, result_of, results, say,
    text, thinking,
};
use crate::test_requests::refusal_of;

fn is_invalid(error: &ClientError, words: &str) -> bool {
    matches!(error, ClientError::InvalidMessages { reason } if reason.contains(words))
}

const LAST: &str = "the last message is an assistant message";

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

#[test]
fn given_two_faults_in_the_history_when_encoded_then_the_first_in_history_order_is_reported() {
    let provider = ContentBlock::ProviderToolCall(ProviderToolCall::new(fresh_block()));
    let error = refusal_of(vec![
        say("hi"),
        earlier(vec![response(vec![ContentBlock::Image(image(
            MediaType::Png,
            "QUJD",
        ))])]),
        say("again"),
        reply(vec![response(vec![provider, text("x")])]),
    ]);
    assert!(is_invalid(&error, "holds an image"), "{error:?}");
}

#[test]
fn given_an_earlier_fault_and_a_last_assistant_message_when_encoded_then_the_earlier_fault_is_reported()
 {
    let error = refusal_of(vec![
        say("hi"),
        earlier(vec![response(vec![ContentBlock::Image(image(
            MediaType::Png,
            "QUJD",
        ))])]),
        say("again"),
        reply(vec![response(vec![text("x")])]),
    ]);
    assert!(is_invalid(&error, "holds an image"), "{error:?}");
}
