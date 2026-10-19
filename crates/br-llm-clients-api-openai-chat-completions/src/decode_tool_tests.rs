use br_llm_clients_common_call::{ProviderBlock, ProviderEvent};
use br_llm_messages::StopReason;
use serde_json::{Value, json};

use crate::test_support::{
    chunk, content_chunk, end, fails, feed, finish_chunk, plain_decoder, run_chunks, start,
    text_delta, tool_chunk,
};

fn call(name: &str, id: Option<&str>) -> ProviderBlock {
    ProviderBlock::ToolCall {
        name: name.to_owned(),
        id: id.map(str::to_owned),
    }
}

fn arguments(key: &str, json: &str) -> ProviderEvent {
    ProviderEvent::ToolArgumentsDelta {
        key: key.to_owned(),
        json: json.to_owned(),
    }
}

fn first(index: u64, id: &str, name: &str) -> String {
    tool_chunk(
        json!([{"index": index, "id": id, "type": "function", "function": {"name": name, "arguments": ""}}]),
        None,
    )
}

fn piece(index: u64, text: &str) -> String {
    tool_chunk(
        json!([{"index": index, "function": {"arguments": text}}]),
        None,
    )
}

#[test]
fn given_a_streamed_tool_call_when_decoded_then_nothing_is_given_before_the_finish() {
    let mut decoder = plain_decoder();
    for data in [
        first(0, "call_1", "lookup"),
        piece(0, "{\"q\":"),
        piece(0, "\"rust\"}"),
    ] {
        let events = feed(decoder.as_mut(), &data).expect("chunk");
        assert!(
            events
                .iter()
                .all(|event| matches!(event, ProviderEvent::Model(_)))
        );
    }
}

#[test]
fn given_a_streamed_tool_call_when_the_finish_arrives_then_the_whole_call_comes_with_one_arguments_delta()
 {
    let events = run_chunks(&[
        first(0, "call_1", "lookup"),
        piece(0, "{\"q\":"),
        piece(0, "\"rust\"}"),
        finish_chunk("tool_calls"),
    ]);
    assert_eq!(
        events,
        [
            start("0", call("lookup", Some("call_1"))),
            arguments("0", "{\"q\":\"rust\"}"),
            end("0"),
            ProviderEvent::Stop(StopReason::AwaitingToolResults),
        ]
    );
}

#[test]
fn given_two_calls_with_interleaved_fragments_when_finished_then_they_come_in_increasing_index_order()
 {
    let events = run_chunks(&[
        first(1, "call_b", "second"),
        first(0, "call_a", "first"),
        piece(1, "{\"b\":2}"),
        piece(0, "{\"a\":1}"),
        finish_chunk("tool_calls"),
    ]);
    assert_eq!(
        events,
        [
            start("0", call("first", Some("call_a"))),
            arguments("0", "{\"a\":1}"),
            end("0"),
            start("1", call("second", Some("call_b"))),
            arguments("1", "{\"b\":2}"),
            end("1"),
            ProviderEvent::Stop(StopReason::AwaitingToolResults),
        ]
    );
}

#[test]
fn given_two_calls_in_one_chunk_when_finished_then_both_are_held_and_given_in_order() {
    let both = tool_chunk(
        json!([
            {"index": 1, "id": "b", "type": "function", "function": {"name": "two", "arguments": "{}"}},
            {"index": 0, "id": "a", "type": "function", "function": {"name": "one", "arguments": "{}"}}
        ]),
        None,
    );
    let events = run_chunks(&[both, finish_chunk("tool_calls")]);
    let keys: Vec<&str> = events
        .iter()
        .filter_map(|event| match event {
            ProviderEvent::BlockStart { key, .. } => Some(key.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(keys, ["0", "1"]);
}

#[test]
fn given_the_name_in_a_later_fragment_when_finished_then_the_call_carries_it() {
    let events = run_chunks(&[
        tool_chunk(
            json!([{"index": 0, "id": "c1", "type": "function", "function": {"arguments": "{"}}]),
            None,
        ),
        tool_chunk(
            json!([{"index": 0, "function": {"name": "late", "arguments": "}"}}]),
            None,
        ),
        finish_chunk("tool_calls"),
    ]);
    assert_eq!(events.first(), Some(&start("0", call("late", Some("c1")))));
    assert_eq!(events.get(1), Some(&arguments("0", "{}")));
}

#[test]
fn given_an_empty_id_and_an_empty_name_when_decoded_then_they_are_read_as_absent() {
    let events = run_chunks(&[
        tool_chunk(
            json!([{"index": 0, "id": "", "function": {"name": "", "arguments": ""}}]),
            None,
        ),
        tool_chunk(
            json!([{"index": 0, "function": {"name": "f", "arguments": "{}"}}]),
            None,
        ),
        finish_chunk("tool_calls"),
    ]);
    assert_eq!(events.first(), Some(&start("0", call("f", None))));
}

#[test]
fn given_nulls_instead_of_absent_members_when_decoded_then_they_are_read_as_absent() {
    let events = run_chunks(&[
        first(0, "c1", "f"),
        tool_chunk(
            json!([{"index": 0, "id": null, "type": null, "function": {"name": null, "arguments": "{}"}}]),
            None,
        ),
        finish_chunk("tool_calls"),
    ]);
    assert_eq!(events.first(), Some(&start("0", call("f", Some("c1")))));
}

#[test]
fn given_the_same_id_and_name_again_when_decoded_then_it_is_accepted() {
    let events = run_chunks(&[
        first(0, "c1", "f"),
        first(0, "c1", "f"),
        piece(0, "{}"),
        finish_chunk("tool_calls"),
    ]);
    assert_eq!(events.first(), Some(&start("0", call("f", Some("c1")))));
}

#[test]
fn given_a_different_id_or_name_for_a_held_index_when_decoded_then_it_is_a_payload_error() {
    let mut decoder = plain_decoder();
    feed(decoder.as_mut(), &first(0, "c1", "f")).expect("first");
    let other_id = feed(decoder.as_mut(), &first(0, "c2", "f")).expect_err("id");
    assert!(other_id.text().contains("id"));
    let other_name = feed(decoder.as_mut(), &first(0, "c1", "g")).expect_err("name");
    assert!(other_name.text().contains("name"));
}

#[test]
fn given_blank_arguments_when_the_finish_is_tool_calls_then_the_arguments_are_an_empty_object() {
    for blank in ["", "  ", "\n"] {
        let events = run_chunks(&[
            first(0, "c1", "f"),
            piece(0, blank),
            finish_chunk("tool_calls"),
        ]);
        assert_eq!(events.get(1), Some(&arguments("0", "{}")), "{blank:?}");
    }
}

#[test]
fn given_no_arguments_member_at_all_when_finished_then_the_arguments_are_an_empty_object() {
    let bare = tool_chunk(
        json!([{"index": 0, "id": "c1", "function": {"name": "f"}}]),
        None,
    );
    let events = run_chunks(&[bare, finish_chunk("stop")]);
    assert_eq!(events.get(1), Some(&arguments("0", "{}")));
}

#[test]
fn given_blank_arguments_when_the_finish_is_length_then_no_arguments_delta_is_given() {
    for blank in ["", "   "] {
        let events = run_chunks(&[first(0, "c1", "f"), piece(0, blank), finish_chunk("length")]);
        assert_eq!(
            events,
            [
                start("0", call("f", Some("c1"))),
                end("0"),
                ProviderEvent::Stop(StopReason::MaxTokens),
            ],
            "{blank:?}"
        );
    }
}

#[test]
fn given_cut_arguments_when_the_finish_is_length_then_they_are_given_as_received() {
    let events = run_chunks(&[
        first(0, "c1", "f"),
        piece(0, "{\"a\":"),
        finish_chunk("length"),
    ]);
    assert_eq!(events.get(1), Some(&arguments("0", "{\"a\":")));
}

#[test]
fn given_arguments_that_are_not_json_when_finished_then_they_are_still_given_as_received() {
    let events = run_chunks(&[
        first(0, "c1", "f"),
        piece(0, "not json"),
        finish_chunk("tool_calls"),
    ]);
    assert_eq!(events.get(1), Some(&arguments("0", "not json")));
}

#[test]
fn given_a_held_call_with_no_name_when_finished_then_it_is_a_payload_error_for_any_reason() {
    for reason in ["tool_calls", "stop", "length", "content_filter", "weird"] {
        let mut decoder = plain_decoder();
        let nameless = tool_chunk(
            json!([{"index": 0, "id": "c1", "function": {"arguments": "{}"}}]),
            None,
        );
        feed(decoder.as_mut(), &nameless).expect("held");
        let error = feed(decoder.as_mut(), &finish_chunk(reason)).expect_err(reason);
        assert!(error.text().contains("name"), "{error}");
    }
}

#[test]
fn given_a_call_with_no_function_member_when_finished_then_it_is_a_payload_error() {
    let mut decoder = plain_decoder();
    feed(
        decoder.as_mut(),
        &tool_chunk(json!([{"index": 0, "id": "c1"}]), None),
    )
    .expect("held");
    assert!(feed(decoder.as_mut(), &finish_chunk("tool_calls")).is_err());
}

#[test]
fn given_text_then_a_tool_call_then_more_text_when_finished_then_all_text_comes_before_the_call() {
    let events = run_chunks(&[
        content_chunk("before "),
        first(0, "c1", "f"),
        content_chunk("after"),
        piece(0, "{}"),
        finish_chunk("tool_calls"),
    ]);
    assert_eq!(
        events,
        [
            start("text", ProviderBlock::Text),
            text_delta("text", "before "),
            text_delta("text", "after"),
            end("text"),
            start("0", call("f", Some("c1"))),
            arguments("0", "{}"),
            end("0"),
            ProviderEvent::Stop(StopReason::AwaitingToolResults),
        ]
    );
}

#[test]
fn given_an_index_that_is_missing_null_or_not_an_integer_when_decoded_then_it_is_a_payload_error() {
    let bad: [Value; 7] = [
        json!(null),
        json!(-1),
        json!(1.5),
        json!("0"),
        json!(true),
        json!([0]),
        json!(1024),
    ];
    for index in bad {
        let data = tool_chunk(json!([{"index": index, "function": {"name": "f"}}]), None);
        assert!(fails(&data).text().contains("index"), "{index}");
    }
    let missing = tool_chunk(json!([{"id": "c", "function": {"name": "f"}}]), None);
    assert!(fails(&missing).text().contains("index"));
}

#[test]
fn given_the_extreme_valid_indexes_when_decoded_then_they_are_accepted() {
    let mut decoder = plain_decoder();
    for index in [0, 1023] {
        let data = tool_chunk(
            json!([{"index": index, "id": "c", "function": {"name": "f"}}]),
            None,
        );
        assert!(feed(decoder.as_mut(), &data).is_ok());
    }
    let events: Vec<ProviderEvent> =
        feed(decoder.as_mut(), &finish_chunk("tool_calls")).expect("finish");
    let keys: Vec<&str> = events
        .iter()
        .filter_map(|event| match event {
            ProviderEvent::BlockStart { key, .. } => Some(key.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(keys, ["0", "1023"]);
}

#[test]
fn given_a_call_of_another_type_when_decoded_then_it_is_a_payload_error() {
    for kind in [
        json!("custom"),
        json!(1),
        json!(["function"]),
        json!({"a": 1}),
    ] {
        let data = tool_chunk(
            json!([{"index": 0, "type": kind, "function": {"name": "f"}}]),
            None,
        );
        assert!(fails(&data).text().contains("function"), "{kind}");
    }
}

#[test]
fn given_a_fragment_that_is_not_an_object_when_decoded_then_it_is_a_payload_error() {
    for fragment in [json!(1), json!("x"), json!(null), json!([])] {
        assert!(
            fails(&tool_chunk(json!([fragment]), None))
                .text()
                .contains("tool_calls")
        );
    }
}

#[test]
fn given_a_function_of_the_wrong_type_when_decoded_then_the_field_is_named() {
    let cases = [
        (json!({"index": 0, "function": 1}), "function"),
        (json!({"index": 0, "function": {"name": 1}}), "name"),
        (
            json!({"index": 0, "function": {"arguments": {}}}),
            "arguments",
        ),
        (
            json!({"index": 0, "id": 5, "function": {"name": "f"}}),
            "id",
        ),
    ];
    for (fragment, field) in cases {
        let error = fails(&tool_chunk(json!([fragment]), None));
        assert!(error.text().contains(field), "{error}");
    }
}

#[test]
fn given_a_held_call_when_the_sentinel_arrives_without_a_finish_then_it_is_dropped_with_complete_alone()
 {
    let mut decoder = plain_decoder();
    feed(decoder.as_mut(), &first(0, "c1", "f")).expect("held");
    assert_eq!(
        feed(decoder.as_mut(), "[DONE]").expect("sentinel"),
        [ProviderEvent::Complete]
    );
}

#[test]
fn given_an_empty_tool_calls_array_when_decoded_then_nothing_is_held() {
    let events = run_chunks(&[chunk(json!({"tool_calls": []}), None), finish_chunk("stop")]);
    assert_eq!(events, [ProviderEvent::Stop(StopReason::EndTurn)]);
}
