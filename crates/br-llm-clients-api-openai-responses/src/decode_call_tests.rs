use br_llm_clients_common_call::ProviderEvent;
use br_llm_messages::StopReason;

use crate::test_decoder::{
    arguments, end, feed, flat, model, plain_decoder, start_call, stop, usage,
};
use crate::test_events::{
    arguments_delta, call_added, call_done, completed, incomplete, message_added, message_done,
    output_done_part, output_part, part_added, text_delta, usage_object,
};

#[test]
fn given_a_function_call_when_streamed_then_nothing_comes_until_the_terminal_event() {
    let mut decoder = plain_decoder();
    let before = flat(
        &mut decoder,
        &[
            call_added(0, "call_Zr1", "get_weather"),
            arguments_delta(0, "{\"city\":"),
            arguments_delta(0, "\"Paris\"}"),
            call_done(
                0,
                "call_Zr1",
                "get_weather",
                "{\"city\":\"Paris\"}",
                Some("completed"),
            ),
        ],
    );
    assert_eq!(before, vec![]);
    let events = feed(&mut decoder, &completed("gpt-5.1", usage_object(3, 4)));
    assert_eq!(
        events,
        vec![
            start_call("0", "get_weather", Some("call_Zr1")),
            arguments("0", "{\"city\":\"Paris\"}"),
            end("0"),
            usage(3, 4),
            model("gpt-5.1"),
            stop(StopReason::EndTurn),
            ProviderEvent::Complete,
        ]
    );
}

#[test]
fn given_a_text_and_two_calls_in_the_wrong_order_when_finished_then_calls_come_last_in_output_index_order()
 {
    let mut decoder = plain_decoder();
    let events = flat(
        &mut decoder,
        &[
            call_added(1, "call_b", "second"),
            call_added(0, "call_a", "first"),
            message_added(2),
            part_added(2, 0, output_part()),
            text_delta(2, 0, "after"),
            call_done(1, "call_b", "second", "{}", Some("completed")),
            call_done(0, "call_a", "first", "{\"x\":1}", Some("completed")),
            message_done(2, Some("completed"), vec![output_done_part("after")]),
            completed("m", usage_object(1, 1)),
        ],
    );
    let position = |wanted: &ProviderEvent| {
        events
            .iter()
            .position(|event| event == wanted)
            .expect("event")
    };
    let text_end = position(&end("2.0"));
    let first = position(&start_call("0", "first", Some("call_a")));
    let second = position(&start_call("1", "second", Some("call_b")));
    let stop_at = position(&stop(StopReason::EndTurn));
    assert!(text_end < first && first < second && second < stop_at);
    assert_eq!(events.last(), Some(&ProviderEvent::Complete));
}

#[test]
fn given_the_held_calls_when_the_terminal_event_comes_then_all_block_events_precede_the_stop() {
    let mut decoder = plain_decoder();
    flat(
        &mut decoder,
        &[
            call_added(0, "c", "n"),
            call_done(0, "c", "n", "{}", Some("completed")),
        ],
    );
    let events = feed(&mut decoder, &completed("m", usage_object(1, 1)));
    let stop_at = events
        .iter()
        .position(|event| matches!(event, ProviderEvent::Stop(_)))
        .expect("stop");
    assert!(events[stop_at..].iter().all(|event| {
        !matches!(
            event,
            ProviderEvent::BlockStart { .. }
                | ProviderEvent::BlockEnd { .. }
                | ProviderEvent::ToolArgumentsDelta { .. }
        )
    }));
}

#[test]
fn given_a_completed_call_with_empty_arguments_when_finished_then_the_arguments_are_an_empty_object()
 {
    let mut decoder = plain_decoder();
    flat(
        &mut decoder,
        &[
            call_added(0, "c", "ping"),
            call_done(0, "c", "ping", "", Some("completed")),
        ],
    );
    let events = feed(&mut decoder, &completed("m", usage_object(1, 1)));
    assert_eq!(events[1], arguments("0", "{}"));
}

#[test]
fn given_a_call_whose_status_is_not_completed_when_finished_then_it_has_no_block_end() {
    for status in [Some("incomplete"), Some("in_progress"), None] {
        let mut decoder = plain_decoder();
        flat(
            &mut decoder,
            &[
                call_added(0, "c", "n"),
                call_done(0, "c", "n", "{\"a\":", status),
            ],
        );
        let events = feed(
            &mut decoder,
            &incomplete(Some("max_output_tokens"), usage_object(1, 1)),
        );
        assert_eq!(
            &events[..2],
            [start_call("0", "n", Some("c")), arguments("0", "{\"a\":")],
            "{status:?}"
        );
        assert!(!events.contains(&end("0")), "{status:?}");
    }
}

#[test]
fn given_a_not_completed_call_with_empty_arguments_when_finished_then_the_empty_text_is_given_unchanged()
 {
    let mut decoder = plain_decoder();
    flat(
        &mut decoder,
        &[
            call_added(0, "c", "n"),
            call_done(0, "c", "n", "", Some("incomplete")),
        ],
    );
    let events = feed(
        &mut decoder,
        &incomplete(Some("max_output_tokens"), usage_object(1, 1)),
    );
    assert_eq!(events[1], arguments("0", ""));
}

#[test]
fn given_a_call_with_no_done_event_when_finished_then_the_arguments_are_the_deltas_with_no_end() {
    let mut decoder = plain_decoder();
    flat(
        &mut decoder,
        &[
            call_added(0, "call_1", "search"),
            arguments_delta(0, "{\"q\":"),
            arguments_delta(0, "\"ru"),
        ],
    );
    let events = feed(
        &mut decoder,
        &incomplete(Some("max_output_tokens"), usage_object(1, 1)),
    );
    assert_eq!(
        &events[..2],
        [
            start_call("0", "search", Some("call_1")),
            arguments("0", "{\"q\":\"ru")
        ]
    );
    assert!(!events.contains(&end("0")));
}

#[test]
fn given_the_done_arguments_and_the_deltas_when_finished_then_the_done_item_wins() {
    let mut decoder = plain_decoder();
    flat(
        &mut decoder,
        &[
            call_added(0, "c", "n"),
            arguments_delta(0, "{\"streamed\":1}"),
            call_done(0, "c", "n", "{\"final\":2}", Some("completed")),
        ],
    );
    let events = feed(&mut decoder, &completed("m", usage_object(1, 1)));
    assert_eq!(events[1], arguments("0", "{\"final\":2}"));
}

#[test]
fn given_a_done_item_with_a_name_and_a_call_id_when_finished_then_they_replace_the_added_ones() {
    let mut decoder = plain_decoder();
    flat(
        &mut decoder,
        &[
            call_added(0, "call_old", "old_name"),
            call_done(0, "call_new", "new_name", "{}", Some("completed")),
        ],
    );
    let events = feed(&mut decoder, &completed("m", usage_object(1, 1)));
    assert_eq!(events[0], start_call("0", "new_name", Some("call_new")));
}

#[test]
fn given_a_done_item_without_name_or_call_id_when_finished_then_the_added_ones_are_used() {
    let mut decoder = plain_decoder();
    let mut done = call_done(0, "x", "x", "{}", Some("completed"));
    done["item"].as_object_mut().expect("item").remove("name");
    done["item"]
        .as_object_mut()
        .expect("item")
        .remove("call_id");
    flat(
        &mut decoder,
        &[call_added(0, "call_added", "added_name"), done],
    );
    let events = feed(&mut decoder, &completed("m", usage_object(1, 1)));
    assert_eq!(events[0], start_call("0", "added_name", Some("call_added")));
}
