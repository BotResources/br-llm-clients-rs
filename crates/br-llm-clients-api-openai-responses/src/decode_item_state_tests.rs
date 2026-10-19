use br_llm_clients_common_call::End;
use serde_json::json;

use crate::test_decoder::{end, end_of, feed, feed_err, flat, plain_decoder, start_text, text};
use crate::test_events::{
    call_added, message_added, message_done, output_done_part, output_part, part_added, text_delta,
};

#[test]
fn given_an_empty_part_when_done_without_delta_then_one_empty_delta_is_given() {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[message_added(0)]);
    let events = feed(
        &mut decoder,
        &message_done(0, Some("completed"), vec![output_done_part("")]),
    );
    assert_eq!(events, vec![start_text("0.0"), text("0.0", ""), end("0.0")]);
}

#[test]
fn given_a_message_with_no_part_when_done_then_no_event_comes() {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[message_added(0)]);
    assert_eq!(
        feed(&mut decoder, &message_done(0, Some("completed"), vec![])),
        vec![]
    );
}

#[test]
fn given_a_part_that_was_streamed_but_is_missing_from_the_done_item_when_done_then_it_is_a_payload_error()
 {
    let mut decoder = plain_decoder();
    flat(
        &mut decoder,
        &[
            message_added(0),
            part_added(0, 0, output_part()),
            part_added(0, 1, output_part()),
        ],
    );
    let error = feed_err(
        &mut decoder,
        &message_done(0, Some("completed"), vec![output_done_part("only one")]),
    );
    assert!(error.text().starts_with("response.output_item.done"));
}

#[test]
fn given_a_part_added_twice_when_decoded_then_it_is_a_payload_error() {
    let mut decoder = plain_decoder();
    flat(
        &mut decoder,
        &[message_added(0), part_added(0, 0, output_part())],
    );
    feed_err(&mut decoder, &part_added(0, 0, output_part()));
}

#[test]
fn given_a_part_of_an_unknown_type_when_added_then_it_is_a_payload_error() {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[message_added(0)]);
    for part in [
        json!({ "type": "output_audio", "data": "x" }),
        json!({ "type": "reasoning_text", "text": "" }),
        json!({ "text": "no type" }),
    ] {
        feed_err(&mut decoder, &part_added(0, 0, part));
    }
}

#[test]
fn given_a_part_added_to_a_function_call_when_decoded_then_it_is_a_payload_error() {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[call_added(0, "call_1", "search")]);
    feed_err(&mut decoder, &part_added(0, 0, output_part()));
}

#[test]
fn given_events_for_an_item_that_was_not_added_when_decoded_then_each_is_a_payload_error() {
    for event in [
        part_added(5, 0, output_part()),
        text_delta(5, 0, "x"),
        message_done(5, Some("completed"), vec![]),
        arguments_delta_for(5),
    ] {
        let mut decoder = plain_decoder();
        let error = feed_err(&mut decoder, &event);
        assert!(error.text().contains("not added"), "{event}");
    }
}

fn arguments_delta_for(index: u64) -> serde_json::Value {
    crate::test_events::arguments_delta(index, "{}")
}

#[test]
fn given_an_item_added_twice_when_decoded_then_it_is_a_payload_error() {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[message_added(0)]);
    feed_err(&mut decoder, &message_added(0));
}

#[test]
fn given_a_delta_after_the_done_event_of_its_item_when_decoded_then_it_is_a_payload_error() {
    let mut decoder = plain_decoder();
    flat(
        &mut decoder,
        &[
            message_added(0),
            message_done(0, Some("completed"), vec![output_done_part("a")]),
        ],
    );
    feed_err(&mut decoder, &text_delta(0, 0, "late"));
}

#[test]
fn given_an_item_done_twice_when_decoded_then_it_is_a_payload_error() {
    let mut decoder = plain_decoder();
    let done = message_done(0, Some("completed"), vec![output_done_part("a")]);
    flat(&mut decoder, &[message_added(0), done.clone()]);
    feed_err(&mut decoder, &done);
}

#[test]
fn given_a_text_delta_for_a_function_call_item_when_decoded_then_it_is_a_payload_error() {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[call_added(0, "call_1", "search")]);
    feed_err(&mut decoder, &text_delta(0, 0, "x"));
}

#[test]
fn given_a_done_item_of_another_kind_than_the_added_one_when_decoded_then_it_is_a_payload_error() {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[message_added(0)]);
    feed_err(
        &mut decoder,
        &crate::test_events::call_done(0, "c", "n", "{}", Some("completed")),
    );
}

#[test]
fn given_the_end_of_the_stream_when_no_terminal_event_came_then_the_marker_is_missing() {
    let mut decoder = plain_decoder();
    flat(
        &mut decoder,
        &[
            message_added(0),
            part_added(0, 0, output_part()),
            text_delta(0, 0, "x"),
        ],
    );
    assert_eq!(end_of(&mut decoder).expect("read"), End::MarkerMissing);
}

#[test]
fn given_an_empty_stream_when_ended_then_the_marker_is_missing() {
    let mut decoder = plain_decoder();
    assert_eq!(end_of(&mut decoder).expect("read"), End::MarkerMissing);
}

#[test]
fn given_a_held_call_when_arguments_stream_then_nothing_is_given_before_the_terminal_event() {
    let mut decoder = plain_decoder();
    let events = flat(
        &mut decoder,
        &[
            call_added(0, "call_1", "search"),
            crate::test_events::arguments_delta(0, "{\"q\""),
        ],
    );
    assert_eq!(events, vec![]);
}
