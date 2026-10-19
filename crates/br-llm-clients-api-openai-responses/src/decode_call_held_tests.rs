use br_llm_clients_common_call::ProviderEvent;

use crate::test_decoder::{
    arguments, end, feed, feed_err, flat, model, plain_decoder, start_call, start_text, text,
};
use crate::test_events::{
    arguments_delta, call_added, call_done, completed, failed, incomplete, message_added,
    message_done, output_done_part, usage_object,
};

#[test]
fn given_a_call_without_any_call_id_when_finished_then_the_id_is_absent() {
    let mut decoder = plain_decoder();
    let mut added = call_added(0, "x", "tool");
    added["item"]
        .as_object_mut()
        .expect("item")
        .remove("call_id");
    flat(&mut decoder, &[added]);
    let events = feed(
        &mut decoder,
        &incomplete(Some("max_output_tokens"), usage_object(1, 1)),
    );
    assert_eq!(events[0], start_call("0", "tool", None));
}

#[test]
fn given_an_empty_call_id_when_finished_then_the_id_is_absent() {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[call_added(0, "", "tool")]);
    let events = feed(
        &mut decoder,
        &incomplete(Some("max_output_tokens"), usage_object(1, 1)),
    );
    assert_eq!(events[0], start_call("0", "tool", None));
}

#[test]
fn given_a_call_whose_name_is_missing_or_empty_when_finished_then_it_is_a_payload_error() {
    for name in [None, Some("")] {
        let mut decoder = plain_decoder();
        let mut added = call_added(0, "c", "x");
        match name {
            Some(name) => added["item"]["name"] = serde_json::json!(name),
            None => {
                added["item"].as_object_mut().expect("item").remove("name");
            }
        }
        flat(&mut decoder, &[added]);
        let error = feed_err(&mut decoder, &completed("m", usage_object(1, 1)));
        assert!(error.text().starts_with("response.completed"), "{name:?}");
    }
}

#[test]
fn given_a_nameless_call_when_the_response_fails_then_the_failure_is_given_not_a_payload_error() {
    let mut decoder = plain_decoder();
    let mut added = call_added(0, "c", "x");
    added["item"].as_object_mut().expect("item").remove("name");
    flat(&mut decoder, &[added]);
    let events = feed(&mut decoder, &failed("server_error", "boom"));
    assert!(matches!(events.last(), Some(ProviderEvent::Error(_))));
}

#[test]
fn given_a_failed_response_when_decoded_then_held_calls_and_stop_and_complete_are_not_given() {
    let mut decoder = plain_decoder();
    flat(
        &mut decoder,
        &[
            call_added(0, "c", "n"),
            call_done(0, "c", "n", "{}", Some("completed")),
        ],
    );
    let events = feed(&mut decoder, &failed("server_error", "boom"));
    assert_eq!(events.len(), 2);
    assert_eq!(events[0], model("gpt-5.1-2026-01-01"));
    assert!(matches!(events[1], ProviderEvent::Error(_)));
}

#[test]
fn given_a_call_done_without_arguments_when_decoded_then_it_is_a_payload_error() {
    let mut decoder = plain_decoder();
    let mut done = call_done(0, "c", "n", "{}", Some("completed"));
    done["item"]
        .as_object_mut()
        .expect("item")
        .remove("arguments");
    flat(&mut decoder, &[call_added(0, "c", "n")]);
    feed_err(&mut decoder, &done);
}

#[test]
fn given_a_call_next_to_text_when_finished_then_the_text_events_come_before_the_call_events() {
    let mut decoder = plain_decoder();
    let events = flat(
        &mut decoder,
        &[
            call_added(0, "c", "n"),
            call_done(0, "c", "n", "{}", Some("completed")),
            message_added(1),
            message_done(1, Some("completed"), vec![output_done_part("hi")]),
            completed("m", usage_object(1, 1)),
        ],
    );
    assert_eq!(
        &events[..6],
        [
            start_text("1.0"),
            text("1.0", "hi"),
            end("1.0"),
            start_call("0", "n", Some("c")),
            arguments("0", "{}"),
            end("0"),
        ]
    );
}

#[test]
fn given_a_function_call_item_with_a_streamed_arguments_delta_that_is_not_text_when_decoded_then_it_is_a_payload_error()
 {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[call_added(0, "c", "n")]);
    let mut event = arguments_delta(0, "x");
    event["delta"] = serde_json::json!(5);
    feed_err(&mut decoder, &event);
}

#[test]
fn given_an_arguments_delta_for_a_message_item_when_decoded_then_it_is_a_payload_error() {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[message_added(0)]);
    feed_err(&mut decoder, &arguments_delta(0, "{}"));
}
