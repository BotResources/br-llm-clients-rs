use serde_json::json;

use crate::test_decoder::{end, feed, feed_err, flat, plain_decoder, start_thinking, thinking};
use crate::test_events::{
    completed, message_added, reasoning_added, reasoning_done, reasoning_text_delta, summary_delta,
    usage_object,
};

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
    let (format, carried) = events
        .iter()
        .find_map(|event| match event {
            br_llm_clients_common_call::ProviderEvent::SignatureDelta {
                format, signature, ..
            } => Some((format.clone(), signature.clone())),
            _ => None,
        })
        .expect("a signature");
    use crate::test_history as h;
    let material =
        br_llm_messages::ReplayMaterial::new(format, br_llm_messages::ReplayPayload::text(carried));
    let call = h::call(Some("c"), "search", "{}");
    let turns = vec![
        h::say("go"),
        h::reply(vec![
            h::response(vec![
                h::thinking("s", Some(material)),
                br_llm_messages::ContentBlock::RuntimeToolCall(call.clone()),
            ]),
            h::results(vec![h::result_of(&call, vec![h::plain_text("r")], false)]),
        ]),
    ];
    let items = crate::test_requests::input_of(turns);
    assert_eq!(
        items[1],
        json!({ "type": "reasoning", "id": "rs_77", "summary": [], "encrypted_content": "ENC==" })
    );
}

#[test]
fn given_reasoning_text_then_a_first_summary_part_with_a_high_index_when_decoded_then_no_separator_comes()
 {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[reasoning_added(0)]);
    feed(&mut decoder, &reasoning_text_delta(0, 0, "raw"));
    assert_eq!(
        feed(&mut decoder, &summary_delta(0, 1, "s")),
        vec![thinking("0", "s")]
    );
}

#[test]
fn given_two_decoders_of_one_dialect_when_one_reads_then_the_other_keeps_no_state() {
    let dialect = crate::test_requests::dialect(br_llm_clients_core::ReasoningLevel::Off);
    let mut first = br_llm_clients_common_call::Dialect::decoder(
        &dialect,
        &br_llm_clients_core::OutputMode::Plain,
    );
    let mut second = br_llm_clients_common_call::Dialect::decoder(
        &dialect,
        &br_llm_clients_core::OutputMode::Plain,
    );
    flat(&mut first, &[reasoning_added(0)]);
    feed(&mut first, &summary_delta(0, 0, "a"));
    assert_eq!(
        feed(&mut second, &crate::test_events::message_added(0)),
        vec![]
    );
    flat(&mut second, &[reasoning_added(1)]);
    assert_eq!(
        feed(&mut second, &summary_delta(1, 5, "b")),
        vec![start_thinking("1"), thinking("1", "b")]
    );
}
