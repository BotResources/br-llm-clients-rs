use br_llm_clients_common_call::ProviderEvent;
use br_llm_messages::StopReason;

use crate::test_decoder::{
    end, feed, flat, start_structured, start_text, stop, structured, structured_decoder, text,
    usage,
};
use crate::test_events::{
    completed, incomplete, message_added, message_done, output_done_part, output_part, part_added,
    refusal_delta, refusal_done_part, refusal_part, text_delta, usage_object,
};

#[test]
fn given_the_structured_mode_when_an_output_text_part_opens_then_it_is_a_structured_block() {
    let mut decoder = structured_decoder();
    let events = flat(
        &mut decoder,
        &[
            message_added(0),
            part_added(0, 0, output_part()),
            text_delta(0, 0, "{\"value\":"),
        ],
    );
    assert_eq!(
        events,
        vec![start_structured("0.0"), structured("0.0", "{\"value\":")]
    );
}

#[test]
fn given_the_structured_mode_when_a_delta_opens_the_block_then_it_is_a_structured_block() {
    let mut decoder = structured_decoder();
    flat(&mut decoder, &[message_added(0)]);
    assert_eq!(
        feed(&mut decoder, &text_delta(0, 0, "{")),
        vec![start_structured("0.0"), structured("0.0", "{")]
    );
}

#[test]
fn given_a_completed_item_when_done_then_the_structured_block_is_closed() {
    let mut decoder = structured_decoder();
    flat(
        &mut decoder,
        &[message_added(0), text_delta(0, 0, "{\"value\":\"a\"}")],
    );
    let events = feed(
        &mut decoder,
        &message_done(
            0,
            Some("completed"),
            vec![output_done_part("{\"value\":\"a\"}")],
        ),
    );
    assert_eq!(events, vec![end("0.0")]);
}

#[test]
fn given_a_status_that_is_not_completed_when_done_then_the_structured_block_stays_open() {
    for status in [
        Some("incomplete"),
        Some("in_progress"),
        Some("failed"),
        None,
    ] {
        let mut decoder = structured_decoder();
        flat(
            &mut decoder,
            &[message_added(0), text_delta(0, 0, "{\"val")],
        );
        let events = feed(
            &mut decoder,
            &message_done(0, status, vec![output_done_part("{\"val")]),
        );
        assert_eq!(events, vec![], "{status:?}");
    }
}

#[test]
fn given_a_done_item_with_no_delta_when_structured_then_the_json_is_one_structured_delta() {
    let mut decoder = structured_decoder();
    flat(&mut decoder, &[message_added(0)]);
    let events = feed(
        &mut decoder,
        &message_done(
            0,
            Some("completed"),
            vec![output_done_part("{\"value\":1}")],
        ),
    );
    assert_eq!(
        events,
        vec![
            start_structured("0.0"),
            structured("0.0", "{\"value\":1}"),
            end("0.0")
        ]
    );
}

#[test]
fn given_the_response_ends_with_an_unclosed_structured_item_then_the_block_is_not_closed() {
    let mut decoder = structured_decoder();
    flat(&mut decoder, &[message_added(0), text_delta(0, 0, "{\"va")]);
    let events = feed(
        &mut decoder,
        &incomplete(Some("max_output_tokens"), usage_object(5, 6)),
    );
    assert!(!events.contains(&end("0.0")));
    assert!(events.contains(&stop(StopReason::MaxTokens)));
}

#[test]
fn given_the_structured_mode_when_a_refusal_comes_then_it_is_always_a_text_block_and_a_text_delta()
{
    let mut decoder = structured_decoder();
    let events = flat(
        &mut decoder,
        &[
            message_added(0),
            part_added(0, 0, refusal_part()),
            refusal_delta(0, 0, "I cannot"),
            message_done(0, Some("completed"), vec![refusal_done_part("I cannot")]),
            completed("m", usage_object(1, 1)),
        ],
    );
    assert_eq!(
        &events[..3],
        [start_text("0.0"), text("0.0", "I cannot"), end("0.0")]
    );
    assert!(events.contains(&stop(StopReason::Refusal)));
}

#[test]
fn given_the_structured_mode_when_a_refusal_delta_opens_the_block_then_it_is_a_text_block() {
    let mut decoder = structured_decoder();
    flat(&mut decoder, &[message_added(0)]);
    assert_eq!(
        feed(&mut decoder, &refusal_delta(0, 0, "no")),
        vec![start_text("0.0"), text("0.0", "no")]
    );
}

#[test]
fn given_the_structured_mode_and_a_text_and_a_refusal_in_one_message_then_each_keeps_its_kind() {
    let mut decoder = structured_decoder();
    let events = flat(
        &mut decoder,
        &[
            message_added(0),
            message_done(
                0,
                Some("completed"),
                vec![output_done_part("{}"), refusal_done_part("but no")],
            ),
        ],
    );
    assert_eq!(
        events,
        vec![
            start_structured("0.0"),
            structured("0.0", "{}"),
            end("0.0"),
            start_text("0.1"),
            text("0.1", "but no"),
            end("0.1"),
        ]
    );
}

#[test]
fn given_a_completed_structured_reply_when_finished_then_the_stop_is_end_turn_with_usage_and_complete()
 {
    let mut decoder = structured_decoder();
    let events = feed(&mut decoder, &completed("gpt-5.1", usage_object(10, 20)));
    assert_eq!(
        events,
        vec![
            usage(10, 20),
            ProviderEvent::Model("gpt-5.1".to_owned()),
            stop(StopReason::EndTurn),
            ProviderEvent::Complete
        ]
    );
}
