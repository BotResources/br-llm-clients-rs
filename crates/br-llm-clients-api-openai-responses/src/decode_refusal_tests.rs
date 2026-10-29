use br_llm_clients_common_call::ProviderEvent;
use br_llm_messages::ResponseEnding;

use crate::test_decoder::{end, feed, flat, plain_decoder, start_text, text};
use crate::test_events::{
    completed, incomplete, message_added, message_done, output_part, part_added, refusal_delta,
    refusal_done_part, refusal_part, text_delta, usage_object,
};

fn stop_of(events: &[ProviderEvent]) -> ResponseEnding {
    events
        .iter()
        .find_map(|event| match event {
            ProviderEvent::Stop(reason) => Some(reason.clone()),
            _ => None,
        })
        .expect("a stop event")
}

#[test]
fn given_a_refusal_part_when_added_and_streamed_then_it_is_a_text_block() {
    let mut decoder = plain_decoder();
    let events = flat(
        &mut decoder,
        &[
            message_added(0),
            part_added(0, 0, refusal_part()),
            refusal_delta(0, 0, "I can't help"),
            refusal_delta(0, 0, " with that."),
        ],
    );
    assert_eq!(
        events,
        vec![
            start_text("0.0"),
            text("0.0", "I can't help"),
            text("0.0", " with that.")
        ]
    );
}

#[test]
fn given_a_refusal_in_a_completed_response_when_finished_then_the_stop_is_refusal() {
    let mut decoder = plain_decoder();
    flat(
        &mut decoder,
        &[
            message_added(0),
            part_added(0, 0, refusal_part()),
            message_done(0, Some("completed"), vec![refusal_done_part("No.")]),
        ],
    );
    let events = feed(&mut decoder, &completed("m", usage_object(1, 1)));
    assert_eq!(stop_of(&events), ResponseEnding::refusal());
}

#[test]
fn given_a_refusal_delta_alone_when_finished_then_the_stop_is_refusal() {
    let mut decoder = plain_decoder();
    flat(
        &mut decoder,
        &[message_added(0), refusal_delta(0, 0, "No.")],
    );
    let events = feed(&mut decoder, &completed("m", usage_object(1, 1)));
    assert_eq!(stop_of(&events), ResponseEnding::refusal());
}

#[test]
fn given_a_refusal_part_only_in_the_done_item_when_finished_then_the_stop_is_refusal() {
    let mut decoder = plain_decoder();
    let events = flat(
        &mut decoder,
        &[
            message_added(0),
            message_done(0, Some("completed"), vec![refusal_done_part("No.")]),
        ],
    );
    assert_eq!(
        events,
        vec![start_text("0.0"), text("0.0", "No."), end("0.0")]
    );
    let finished = feed(&mut decoder, &completed("m", usage_object(1, 1)));
    assert_eq!(stop_of(&finished), ResponseEnding::refusal());
}

#[test]
fn given_a_refusal_part_added_alone_when_finished_then_the_stop_is_refusal() {
    let mut decoder = plain_decoder();
    flat(
        &mut decoder,
        &[message_added(0), part_added(0, 0, refusal_part())],
    );
    let events = feed(&mut decoder, &completed("m", usage_object(1, 1)));
    assert_eq!(stop_of(&events), ResponseEnding::refusal());
}

#[test]
fn given_no_refusal_when_a_response_completes_then_the_stop_is_end_turn() {
    let mut decoder = plain_decoder();
    flat(
        &mut decoder,
        &[
            message_added(0),
            part_added(0, 0, output_part()),
            text_delta(0, 0, "fine"),
        ],
    );
    let events = feed(&mut decoder, &completed("m", usage_object(1, 1)));
    assert_eq!(stop_of(&events), ResponseEnding::finished());
}

#[test]
fn given_a_refusal_in_an_incomplete_response_when_finished_then_the_reason_alone_gives_the_stop() {
    let mut decoder = plain_decoder();
    let events = flat(
        &mut decoder,
        &[
            message_added(0),
            part_added(0, 0, refusal_part()),
            refusal_delta(0, 0, "No."),
            message_done(0, Some("incomplete"), vec![refusal_done_part("No.")]),
            incomplete(Some("max_output_tokens"), usage_object(1, 1)),
        ],
    );
    assert_eq!(stop_of(&events), ResponseEnding::output_exhaustion());
    assert!(events.contains(&text("0.0", "No.")));
    assert!(events.contains(&start_text("0.0")));
}

#[test]
fn given_a_refusal_in_an_incomplete_response_without_reason_when_finished_then_the_stop_is_other() {
    let mut decoder = plain_decoder();
    flat(
        &mut decoder,
        &[message_added(0), refusal_delta(0, 0, "No.")],
    );
    let events = feed(&mut decoder, &incomplete(None, usage_object(1, 1)));
    assert_eq!(
        stop_of(&events),
        crate::test_decoder::unclassified("incomplete")
    );
}

#[test]
fn given_a_refusal_delta_on_a_part_opened_as_output_text_when_decoded_then_it_is_a_payload_error() {
    let mut decoder = plain_decoder();
    flat(
        &mut decoder,
        &[message_added(0), part_added(0, 0, output_part())],
    );
    crate::test_decoder::feed_err(&mut decoder, &refusal_delta(0, 0, "x"));
}

#[test]
fn given_an_output_text_delta_on_a_part_opened_as_refusal_when_decoded_then_it_is_a_payload_error()
{
    let mut decoder = plain_decoder();
    flat(
        &mut decoder,
        &[message_added(0), part_added(0, 0, refusal_part())],
    );
    crate::test_decoder::feed_err(&mut decoder, &text_delta(0, 0, "x"));
}

#[test]
fn given_a_done_part_of_another_kind_than_the_streamed_one_when_done_then_it_is_a_payload_error() {
    let mut decoder = plain_decoder();
    flat(
        &mut decoder,
        &[message_added(0), part_added(0, 0, output_part())],
    );
    crate::test_decoder::feed_err(
        &mut decoder,
        &message_done(0, Some("completed"), vec![refusal_done_part("x")]),
    );
}

#[test]
fn given_a_refusal_part_text_in_the_field_text_when_done_then_it_is_a_payload_error() {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[message_added(0)]);
    let wrong = serde_json::json!({ "type": "refusal", "text": "x" });
    crate::test_decoder::feed_err(
        &mut decoder,
        &message_done(0, Some("completed"), vec![wrong]),
    );
}
