use br_llm_clients_common_call::Payload;
use br_llm_messages::StopReason;
use serde_json::json;

use crate::test_decoder::{
    end, feed, feed_err, flat, model, plain_decoder, run_all, start_text, stop, text, usage,
};
use crate::test_events::{
    completed, created, in_progress, message_added, message_done, output_done_part, output_part,
    part_added, text_delta, usage_object,
};

#[test]
fn given_a_documented_text_stream_when_decoded_then_each_frame_gives_the_expected_events() {
    let mut decoder = plain_decoder();
    let produced = run_all(
        &mut decoder,
        &[
            created("gpt-5.1-2026-01-01"),
            in_progress(),
            message_added(0),
            part_added(0, 0, output_part()),
            text_delta(0, 0, "Hi"),
            text_delta(0, 0, " there"),
            json!({"type":"response.output_text.done","item_id":"msg_0b1c","output_index":0,"content_index":0,"text":"Hi there","logprobs":[],"sequence_number":6}),
            json!({"type":"response.content_part.done","item_id":"msg_0b1c","output_index":0,"content_index":0,"part":output_done_part("Hi there"),"sequence_number":7}),
            message_done(0, Some("completed"), vec![output_done_part("Hi there")]),
            completed("gpt-5.1-2026-01-01", usage_object(37, 11)),
        ],
    );
    assert_eq!(
        produced,
        vec![
            vec![model("gpt-5.1-2026-01-01")],
            vec![],
            vec![],
            vec![start_text("0.0")],
            vec![text("0.0", "Hi")],
            vec![text("0.0", " there")],
            vec![],
            vec![],
            vec![end("0.0")],
            vec![
                usage(37, 11),
                model("gpt-5.1-2026-01-01"),
                stop(StopReason::EndTurn),
                br_llm_clients_common_call::ProviderEvent::Complete
            ],
        ]
    );
}

#[test]
fn given_an_event_name_that_disagrees_with_the_type_when_decoded_then_the_type_wins() {
    let mut decoder = plain_decoder();
    let data = created("m").to_string();
    for name in [
        None,
        Some("message".to_owned()),
        Some("response.failed".to_owned()),
    ] {
        let events = decoder
            .payload(Payload::new(name, data.clone()))
            .expect("read");
        assert_eq!(events, vec![model("m")]);
    }
}

#[test]
fn given_an_empty_or_blank_data_when_decoded_then_no_event_comes_and_nothing_fails() {
    let mut decoder = plain_decoder();
    for data in ["", " ", "\n", "\t \r\n"] {
        assert_eq!(
            decoder.payload(Payload::new(None, data)).expect("read"),
            vec![]
        );
    }
}

#[test]
fn given_an_empty_model_when_created_then_no_model_event_comes() {
    let mut decoder = plain_decoder();
    assert_eq!(feed(&mut decoder, &created("")), vec![]);
}

#[test]
fn given_a_created_event_without_a_model_when_decoded_then_no_event_comes() {
    let mut decoder = plain_decoder();
    let event = json!({ "type": "response.created", "response": { "status": "in_progress" } });
    assert_eq!(feed(&mut decoder, &event), vec![]);
}

#[test]
fn given_events_this_library_does_not_list_when_decoded_then_they_are_ignored() {
    let mut decoder = plain_decoder();
    for kind in [
        "response.queued",
        "response.in_progress",
        "response.output_text.done",
        "response.content_part.done",
        "response.refusal.done",
        "response.reasoning_summary_part.added",
        "response.reasoning_summary_part.done",
        "response.reasoning_summary_text.done",
        "response.reasoning_text.done",
        "response.function_call_arguments.done",
        "response.output_text.annotation.added",
        "response.web_search_call.in_progress",
        "response.future.event",
        "ping",
    ] {
        let event =
            json!({ "type": kind, "output_index": 99, "item_id": "x", "sequence_number": 1 });
        assert_eq!(feed(&mut decoder, &event), vec![], "{kind}");
    }
}

#[test]
fn given_metadata_fields_when_decoded_then_they_change_nothing() {
    let mut decoder = plain_decoder();
    flat(
        &mut decoder,
        &[message_added(0), part_added(0, 0, output_part())],
    );
    let mut event = text_delta(0, 0, "x");
    event["sequence_number"] = json!(41);
    event["obfuscation"] = json!("0123456789");
    event["logprobs"] = json!([{ "token": "x", "logprob": -0.1 }]);
    event["annotations"] = json!([{ "type": "url_citation" }]);
    event["unknown_future_field"] = json!({ "a": 1 });
    assert_eq!(feed(&mut decoder, &event), vec![text("0.0", "x")]);
}

#[test]
fn given_two_parts_and_two_messages_when_decoded_then_keys_are_output_index_dot_content_index() {
    let mut decoder = plain_decoder();
    let events = flat(
        &mut decoder,
        &[
            message_added(0),
            part_added(0, 0, output_part()),
            part_added(0, 1, output_part()),
            message_added(3),
            part_added(3, 7, output_part()),
            text_delta(0, 1, "b"),
            text_delta(3, 7, "c"),
            text_delta(0, 0, "a"),
        ],
    );
    assert_eq!(
        events,
        vec![
            start_text("0.0"),
            start_text("0.1"),
            start_text("3.7"),
            text("0.1", "b"),
            text("3.7", "c"),
            text("0.0", "a"),
        ]
    );
}

#[test]
fn given_a_delta_before_the_part_was_added_when_decoded_then_the_block_is_opened_first() {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[message_added(0)]);
    assert_eq!(
        feed(&mut decoder, &text_delta(0, 0, "Hel")),
        vec![start_text("0.0"), text("0.0", "Hel")]
    );
    assert_eq!(
        feed(&mut decoder, &text_delta(0, 0, "lo")),
        vec![text("0.0", "lo")]
    );
}

#[test]
fn given_a_done_item_with_no_delta_when_decoded_then_the_text_is_given_as_one_delta_then_closed() {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[message_added(0)]);
    let events = feed(
        &mut decoder,
        &message_done(0, Some("completed"), vec![output_done_part("whole text")]),
    );
    assert_eq!(
        events,
        vec![start_text("0.0"), text("0.0", "whole text"), end("0.0")]
    );
}

#[test]
fn given_a_part_added_and_no_delta_when_done_then_the_part_text_is_given_once() {
    let mut decoder = plain_decoder();
    flat(
        &mut decoder,
        &[message_added(0), part_added(0, 0, output_part())],
    );
    let events = feed(
        &mut decoder,
        &message_done(0, Some("completed"), vec![output_done_part("abc")]),
    );
    assert_eq!(events, vec![text("0.0", "abc"), end("0.0")]);
}

#[test]
fn given_deltas_equal_to_the_done_text_when_done_then_only_the_end_comes() {
    let mut decoder = plain_decoder();
    flat(
        &mut decoder,
        &[
            message_added(0),
            part_added(0, 0, output_part()),
            text_delta(0, 0, "ab"),
            text_delta(0, 0, "c"),
        ],
    );
    let events = feed(
        &mut decoder,
        &message_done(0, Some("completed"), vec![output_done_part("abc")]),
    );
    assert_eq!(events, vec![end("0.0")]);
}

#[test]
fn given_deltas_that_differ_from_the_done_text_when_done_then_it_is_a_payload_error() {
    let mut decoder = plain_decoder();
    flat(
        &mut decoder,
        &[
            message_added(0),
            part_added(0, 0, output_part()),
            text_delta(0, 0, "abc"),
        ],
    );
    let error = feed_err(
        &mut decoder,
        &message_done(0, Some("completed"), vec![output_done_part("abd")]),
    );
    assert!(error.text().starts_with("response.output_item.done"));
}

#[test]
fn given_a_text_block_whatever_the_status_when_done_then_it_is_closed() {
    for status in [
        Some("completed"),
        Some("incomplete"),
        Some("in_progress"),
        None,
    ] {
        let mut decoder = plain_decoder();
        flat(&mut decoder, &[message_added(0)]);
        let events = feed(
            &mut decoder,
            &message_done(0, status, vec![output_done_part("t")]),
        );
        assert_eq!(events.last(), Some(&end("0.0")), "{status:?}");
    }
}

#[test]
fn given_an_item_with_no_done_event_when_the_response_completes_then_its_text_block_is_closed() {
    let mut decoder = plain_decoder();
    flat(
        &mut decoder,
        &[
            message_added(0),
            part_added(0, 0, output_part()),
            text_delta(0, 0, "cut"),
        ],
    );
    let events = feed(&mut decoder, &completed("m", usage_object(1, 1)));
    assert_eq!(events.first(), Some(&end("0.0")));
}
