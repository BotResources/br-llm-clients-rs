use serde_json::json;

use crate::test_decoder::{end, feed, feed_err, flat, plain_decoder, start_thinking, thinking};
use crate::test_events::{
    completed, message_added, part_added, reasoning_added, reasoning_done, reasoning_text_delta,
    summary_delta, summary_part_added, usage_object,
};

fn envelope(id: &str, encrypted: &str) -> String {
    crate::test_requests::envelope_text(id, encrypted)
}

fn parsed(text: &str) -> serde_json::Value {
    serde_json::from_str(text).expect("the signature is JSON")
}

#[test]
fn given_a_summary_stream_when_decoded_then_the_block_opens_at_the_first_delta() {
    let mut decoder = plain_decoder();
    flat(
        &mut decoder,
        &[reasoning_added(0), summary_part_added(0, 0)],
    );
    assert_eq!(
        feed(&mut decoder, &summary_delta(0, 0, "**Greeting**")),
        vec![start_thinking("0"), thinking("0", "**Greeting**")]
    );
    assert_eq!(
        feed(&mut decoder, &summary_delta(0, 0, " the user")),
        vec![thinking("0", " the user")]
    );
}

#[test]
fn given_the_summary_part_added_event_when_decoded_then_it_is_ignored() {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[reasoning_added(0)]);
    assert_eq!(feed(&mut decoder, &summary_part_added(0, 0)), vec![]);
    assert_eq!(feed(&mut decoder, &summary_part_added(0, 5)), vec![]);
}

#[test]
fn given_a_higher_summary_index_after_text_when_decoded_then_two_line_feeds_come_first() {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[reasoning_added(0)]);
    feed(&mut decoder, &summary_delta(0, 0, "one"));
    assert_eq!(
        feed(&mut decoder, &summary_delta(0, 1, "two")),
        vec![thinking("0", "\n\n"), thinking("0", "two")]
    );
    assert_eq!(
        feed(&mut decoder, &summary_delta(0, 1, " more")),
        vec![thinking("0", " more")]
    );
    assert_eq!(
        feed(&mut decoder, &summary_delta(0, 2, "three")),
        vec![thinking("0", "\n\n"), thinking("0", "three")]
    );
}

#[test]
fn given_the_first_summary_delta_with_a_high_index_when_decoded_then_no_separator_comes() {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[reasoning_added(0)]);
    assert_eq!(
        feed(&mut decoder, &summary_delta(0, 3, "x")),
        vec![start_thinking("0"), thinking("0", "x")]
    );
}

#[test]
fn given_an_index_that_is_not_higher_than_the_highest_when_decoded_then_no_separator_comes() {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[reasoning_added(0)]);
    feed(&mut decoder, &summary_delta(0, 2, "a"));
    assert_eq!(
        feed(&mut decoder, &summary_delta(0, 1, "b")),
        vec![thinking("0", "b")]
    );
    assert_eq!(
        feed(&mut decoder, &summary_delta(0, 2, "c")),
        vec![thinking("0", "c")]
    );
}

#[test]
fn given_a_higher_index_but_only_empty_text_so_far_when_decoded_then_no_separator_comes() {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[reasoning_added(0)]);
    feed(&mut decoder, &summary_delta(0, 0, ""));
    assert_eq!(
        feed(&mut decoder, &summary_delta(0, 1, "x")),
        vec![thinking("0", "x")]
    );
}

#[test]
fn given_whitespace_text_so_far_when_a_higher_index_comes_then_the_separator_comes() {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[reasoning_added(0)]);
    feed(&mut decoder, &summary_delta(0, 0, " "));
    assert_eq!(
        feed(&mut decoder, &summary_delta(0, 1, "x")),
        vec![thinking("0", "\n\n"), thinking("0", "x")]
    );
}

#[test]
fn given_reasoning_text_then_the_first_summary_index_when_decoded_then_the_separator_comes() {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[reasoning_added(0)]);
    feed(&mut decoder, &reasoning_text_delta(0, 0, "x"));
    assert_eq!(
        feed(&mut decoder, &summary_delta(0, 0, "s")),
        vec![thinking("0", "s")]
    );
}

#[test]
fn given_summary_text_then_a_second_summary_index_after_reasoning_text_when_decoded_then_one_separator()
 {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[reasoning_added(0)]);
    feed(&mut decoder, &summary_delta(0, 0, "s0"));
    feed(&mut decoder, &reasoning_text_delta(0, 0, "raw"));
    assert_eq!(
        feed(&mut decoder, &summary_delta(0, 1, "s1")),
        vec![thinking("0", "\n\n"), thinking("0", "s1")]
    );
}

#[test]
fn given_reasoning_text_deltas_when_decoded_then_they_go_to_the_thinking_block_and_never_add_a_separator()
 {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[reasoning_added(0)]);
    assert_eq!(
        feed(&mut decoder, &reasoning_text_delta(0, 0, "think")),
        vec![start_thinking("0"), thinking("0", "think")]
    );
    assert_eq!(
        feed(&mut decoder, &reasoning_text_delta(0, 1, "more")),
        vec![thinking("0", "more")]
    );
    assert_eq!(
        feed(&mut decoder, &reasoning_text_delta(0, 2, "again")),
        vec![thinking("0", "again")]
    );
}

#[test]
fn given_a_reasoning_text_part_added_when_decoded_then_it_is_ignored_and_opens_no_block() {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[reasoning_added(0)]);
    let part = json!({ "type": "reasoning_text", "text": "" });
    assert_eq!(feed(&mut decoder, &part_added(0, 0, part)), vec![]);
}

#[test]
fn given_another_part_type_added_to_a_reasoning_item_when_decoded_then_it_is_a_payload_error() {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[reasoning_added(0)]);
    feed_err(
        &mut decoder,
        &part_added(0, 0, json!({ "type": "output_text", "text": "" })),
    );
}

#[test]
fn given_a_done_item_with_id_and_encrypted_content_when_decoded_then_the_signature_is_the_envelope_and_the_block_closes()
 {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[reasoning_added(0)]);
    feed(&mut decoder, &summary_delta(0, 0, "s"));
    let events = feed(
        &mut decoder,
        &reasoning_done(0, "rs_68a1", Some("gAAAAAB-enc"), &["s"]),
    );
    assert_eq!(events.len(), 2);
    assert_eq!(events[1], end("0"));
    let carried = match events[0].clone() {
        br_llm_clients_common_call::ProviderEvent::SignatureDelta { key, signature } => {
            assert_eq!(key, "0");
            signature
        }
        other => panic!("{other:?}"),
    };
    assert_eq!(
        parsed(&carried),
        json!({ "id": "rs_68a1", "encrypted_content": "gAAAAAB-enc" })
    );
    assert_eq!(
        parsed(&carried),
        parsed(&envelope("rs_68a1", "gAAAAAB-enc"))
    );
}

#[test]
fn given_a_done_item_without_prior_delta_when_decoded_then_the_block_opens_with_the_summary_parts_joined()
 {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[reasoning_added(0)]);
    let events = feed(
        &mut decoder,
        &reasoning_done(0, "rs_1", Some("ENC"), &["first part", "second part"]),
    );
    assert_eq!(events.len(), 4);
    assert_eq!(events[0], start_thinking("0"));
    assert_eq!(events[1], thinking("0", "first part\n\nsecond part"));
    assert_eq!(events[3], end("0"));
    assert!(matches!(
        &events[2],
        br_llm_clients_common_call::ProviderEvent::SignatureDelta { .. }
    ));
}

#[test]
fn given_a_done_item_with_a_signature_and_no_summary_when_decoded_then_a_block_opens_for_the_signature_only()
 {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[reasoning_added(0)]);
    let events = feed(&mut decoder, &reasoning_done(0, "rs_1", Some("ENC"), &[]));
    assert_eq!(events.len(), 3);
    assert_eq!(events[0], start_thinking("0"));
    assert_eq!(events[2], end("0"));
}

#[test]
fn given_a_done_item_with_a_summary_and_no_encrypted_content_when_decoded_then_the_block_has_no_signature()
 {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[reasoning_added(0)]);
    let events = feed(&mut decoder, &reasoning_done(0, "rs_1", None, &["text"]));
    assert_eq!(
        events,
        vec![start_thinking("0"), thinking("0", "text"), end("0")]
    );
}

#[test]
fn given_a_done_item_with_neither_signature_nor_summary_when_decoded_then_no_block_comes() {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[reasoning_added(0)]);
    assert_eq!(
        feed(&mut decoder, &reasoning_done(0, "rs_1", None, &[])),
        vec![]
    );
}

#[test]
fn given_an_empty_id_or_empty_encrypted_content_when_done_then_no_signature_is_given() {
    for (id, encrypted) in [("", Some("ENC")), ("rs_1", Some("")), ("", Some(""))] {
        let mut decoder = plain_decoder();
        flat(&mut decoder, &[reasoning_added(0)]);
        feed(&mut decoder, &summary_delta(0, 0, "t"));
        let events = feed(&mut decoder, &reasoning_done(0, id, encrypted, &["t"]));
        assert_eq!(events, vec![end("0")], "{id:?} {encrypted:?}");
    }
}

#[test]
fn given_a_null_encrypted_content_when_done_then_it_is_read_as_absent() {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[reasoning_added(0)]);
    let mut done = reasoning_done(0, "rs_1", None, &["t"]);
    done["item"]["encrypted_content"] = serde_json::Value::Null;
    assert_eq!(
        feed(&mut decoder, &done),
        vec![start_thinking("0"), thinking("0", "t"), end("0")]
    );
}

#[test]
fn given_the_added_item_with_an_id_when_the_done_item_has_none_then_the_signature_is_built_from_the_done_item_only()
 {
    let mut decoder = plain_decoder();
    let mut added = reasoning_added(0);
    added["item"]["encrypted_content"] = json!("FROM-ADDED");
    flat(&mut decoder, &[added]);
    feed(&mut decoder, &summary_delta(0, 0, "t"));
    let events = feed(&mut decoder, &reasoning_done(0, "rs_68a1", None, &["t"]));
    assert_eq!(events, vec![end("0")]);
}

#[test]
fn given_deltas_when_done_then_the_block_text_is_the_deltas_and_the_done_summary_is_not_added() {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[reasoning_added(0)]);
    let streamed = feed(&mut decoder, &summary_delta(0, 0, "streamed"));
    let done = feed(
        &mut decoder,
        &reasoning_done(0, "rs_1", Some("E"), &["different summary"]),
    );
    assert_eq!(
        streamed,
        vec![start_thinking("0"), thinking("0", "streamed")]
    );
    assert!(done.iter().all(|event| !matches!(
        event,
        br_llm_clients_common_call::ProviderEvent::ThinkingDelta { .. }
    )));
}

#[test]
fn given_a_reasoning_item_with_no_done_event_when_the_response_completes_then_the_block_closes_with_no_signature()
 {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[reasoning_added(0)]);
    feed(&mut decoder, &summary_delta(0, 0, "cut off"));
    let events = feed(&mut decoder, &completed("m", usage_object(1, 1)));
    assert_eq!(events.first(), Some(&end("0")));
    assert!(events.iter().all(|event| !matches!(
        event,
        br_llm_clients_common_call::ProviderEvent::SignatureDelta { .. }
    )));
}

#[test]
fn given_a_reasoning_item_that_never_opened_when_the_response_completes_then_it_gives_no_block() {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[reasoning_added(0)]);
    let events = feed(&mut decoder, &completed("m", usage_object(1, 1)));
    assert!(events.iter().all(|event| !matches!(
        event,
        br_llm_clients_common_call::ProviderEvent::BlockStart { .. }
            | br_llm_clients_common_call::ProviderEvent::BlockEnd { .. }
    )));
}

#[test]
fn given_a_summary_delta_for_a_message_item_when_decoded_then_it_is_a_payload_error() {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[message_added(0)]);
    feed_err(&mut decoder, &summary_delta(0, 0, "x"));
    feed_err(&mut decoder, &reasoning_text_delta(0, 0, "x"));
}

#[test]
fn given_a_summary_delta_without_a_summary_index_when_decoded_then_it_is_a_payload_error() {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[reasoning_added(0)]);
    let mut event = summary_delta(0, 0, "x");
    event
        .as_object_mut()
        .expect("object")
        .remove("summary_index");
    feed_err(&mut decoder, &event);
}

#[test]
fn given_a_summary_part_of_an_unknown_type_when_done_then_it_is_a_payload_error() {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[reasoning_added(0)]);
    let mut done = reasoning_done(0, "rs_1", Some("E"), &[]);
    done["item"]["summary"] = json!([{ "type": "other", "text": "x" }]);
    feed_err(&mut decoder, &done);
}

#[test]
fn given_a_reasoning_item_with_a_signature_when_replayed_in_a_request_then_the_envelope_round_trips()
 {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[reasoning_added(0)]);
    let events = feed(
        &mut decoder,
        &reasoning_done(0, "rs_77", Some("ENC=="), &["s"]),
    );
    let carried = events
        .iter()
        .find_map(|event| match event {
            br_llm_clients_common_call::ProviderEvent::SignatureDelta { signature, .. } => {
                Some(signature.clone())
            }
            _ => None,
        })
        .expect("a signature");
    let messages = vec![
        crate::test_requests::user_text("go"),
        crate::test_requests::assistant(vec![
            crate::test_requests::thinking("s", Some(&carried)),
            crate::test_requests::call_block("c", "search", json!({})),
        ]),
        crate::test_requests::user(vec![crate::test_requests::result_block(
            "c",
            "search",
            &["r"],
            false,
        )]),
    ];
    let items = crate::test_requests::input_of(messages);
    assert_eq!(
        items[1],
        json!({ "type": "reasoning", "id": "rs_77", "summary": [], "encrypted_content": "ENC==" })
    );
}
