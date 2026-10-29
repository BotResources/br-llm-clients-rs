use br_llm_clients_common_call::{ProviderBlock, ProviderEvent};
use br_llm_messages::ResponseEnding;
use serde_json::{Value, json};

use crate::test_support::{
    end, fails, feed, finish_chunk, plain_decoder, run_chunks, start, tool_chunk,
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
            ProviderEvent::Stop(ResponseEnding::tool_use()),
        ]
    );
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
    assert!(
        other_id
            .text()
            .contains("`choices[].delta.tool_calls[].id`")
    );
    let other_name = feed(decoder.as_mut(), &first(0, "c1", "g")).expect_err("name");
    assert!(
        other_name
            .text()
            .contains("`choices[].delta.tool_calls[].function.name`")
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
        assert!(
            fails(&data)
                .text()
                .contains("`choices[].delta.tool_calls[].index`"),
            "{index}"
        );
    }
    let missing = tool_chunk(json!([{"id": "c", "function": {"name": "f"}}]), None);
    assert!(
        fails(&missing)
            .text()
            .contains("`choices[].delta.tool_calls[].index`")
    );
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
        assert!(
            fails(&data)
                .text()
                .contains("`choices[].delta.tool_calls[].type`"),
            "{kind}"
        );
    }
}

#[test]
fn given_a_fragment_that_is_not_an_object_when_decoded_then_it_is_a_payload_error() {
    for fragment in [json!(1), json!("x"), json!(null), json!([])] {
        assert!(
            fails(&tool_chunk(json!([fragment]), None))
                .text()
                .contains("`choices[].delta.tool_calls[]`")
        );
    }
}

#[test]
fn given_a_function_of_the_wrong_type_when_decoded_then_the_field_is_named() {
    let cases = [
        (json!({"index": 0, "function": 1}), "function"),
        (
            json!({"index": 0, "function": {"name": 1}}),
            "function.name",
        ),
        (
            json!({"index": 0, "function": {"arguments": {}}}),
            "function.arguments",
        ),
        (
            json!({"index": 0, "id": 5, "function": {"name": "f"}}),
            "id",
        ),
    ];
    for (fragment, field) in cases {
        let error = fails(&tool_chunk(json!([fragment]), None));
        let place = format!("`choices[].delta.tool_calls[].{field}`");
        assert!(error.text().contains(&place), "{error}");
    }
}
