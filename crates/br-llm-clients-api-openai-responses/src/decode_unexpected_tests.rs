use br_llm_clients_common_call::{Payload, PayloadDecoder};
use serde_json::json;

use crate::test_decoder::{feed_err, flat, plain_decoder};
use crate::test_events::{
    call_added, message_added, message_done, output_done_part, output_part, part_added,
    reasoning_added, reasoning_done, text_delta,
};

const SENTINEL: &str = "SENTINEL-prompt-content-7f3a";

fn error_text(decoder: &mut Box<dyn PayloadDecoder>, data: &str) -> String {
    decoder
        .payload(Payload::new(None, data))
        .expect_err("a payload error")
        .text()
        .to_owned()
}

#[test]
fn given_data_that_is_not_json_when_decoded_then_it_is_a_payload_error_that_never_quotes_it() {
    let mut decoder = plain_decoder();
    for data in [
        format!("not json {SENTINEL}"),
        format!("{{\"type\": \"{SENTINEL}\" "),
        format!("[DONE] {SENTINEL}"),
        format!("{{\"a\":\"{SENTINEL}\",}}"),
    ] {
        let text = error_text(&mut decoder, &data);
        assert!(!text.contains(SENTINEL), "{text}");
        assert!(!text.is_empty());
    }
}

#[test]
fn given_json_with_no_type_text_when_decoded_then_it_is_a_payload_error_that_never_quotes_it() {
    let mut decoder = plain_decoder();
    for value in [
        json!({ "delta": SENTINEL }),
        json!({ "type": 5, "delta": SENTINEL }),
        json!({ "type": null }),
        json!([SENTINEL]),
        json!(SENTINEL),
        json!(42),
        json!(null),
    ] {
        let text = error_text(&mut decoder, &value.to_string());
        assert!(!text.contains(SENTINEL), "{text}");
    }
}

#[test]
fn given_an_item_of_an_unknown_kind_when_added_then_it_is_a_payload_error_naming_the_event_only() {
    for kind in [
        "web_search_call",
        "file_search_call",
        "code_interpreter_call",
        "computer_call",
        "mcp_call",
        "image_generation_call",
        "custom_tool_call",
        "local_shell_call",
        "function_call_output",
        SENTINEL,
    ] {
        let mut decoder = plain_decoder();
        let event = json!({
            "type": "response.output_item.added", "output_index": 0,
            "item": { "id": "x", "type": kind, "status": "in_progress" }
        });
        let text = feed_err(&mut decoder, &event).text().to_owned();
        assert!(text.starts_with("response.output_item.added"));
        assert!(!text.contains(SENTINEL));
        assert!(!text.contains("_call"), "{text}");
    }
}

#[test]
fn given_a_message_item_of_another_role_when_added_then_it_is_a_payload_error() {
    for role in [Some("user"), Some("system"), Some("developer"), None] {
        let mut decoder = plain_decoder();
        let mut item = json!({ "id": "x", "type": "message", "content": [] });
        if let Some(role) = role {
            item["role"] = json!(role);
        }
        feed_err(
            &mut decoder,
            &json!({ "type": "response.output_item.added", "output_index": 0, "item": item }),
        );
    }
}

#[test]
fn given_events_with_wrong_typed_indices_when_decoded_then_each_is_a_payload_error() {
    for index in [
        json!(-1),
        json!(1.5),
        json!("0"),
        json!(null),
        json!(true),
        json!([0]),
    ] {
        for field in ["output_index", "content_index"] {
            let mut decoder = plain_decoder();
            flat(
                &mut decoder,
                &[message_added(0), part_added(0, 0, output_part())],
            );
            let mut event = text_delta(0, 0, "x");
            event[field] = index.clone();
            feed_err(&mut decoder, &event);
        }
    }
}

#[test]
fn given_a_delta_that_is_not_a_text_when_decoded_then_it_is_a_payload_error() {
    for delta in [json!(null), json!(5), json!(["a"]), json!({ "a": 1 })] {
        let mut decoder = plain_decoder();
        flat(&mut decoder, &[message_added(0)]);
        let mut event = text_delta(0, 0, "x");
        event["delta"] = delta;
        feed_err(&mut decoder, &event);
    }
}

#[test]
fn given_a_text_that_differs_from_its_done_item_when_decoded_then_the_error_never_quotes_either_text()
 {
    let mut decoder = plain_decoder();
    flat(
        &mut decoder,
        &[
            message_added(0),
            part_added(0, 0, output_part()),
            text_delta(0, 0, SENTINEL),
        ],
    );
    let text = feed_err(
        &mut decoder,
        &message_done(
            0,
            Some("completed"),
            vec![output_done_part("OTHER-SENTINEL-text")],
        ),
    )
    .text()
    .to_owned();
    assert!(!text.contains(SENTINEL) && !text.contains("OTHER-SENTINEL"));
}

#[test]
fn given_a_done_content_that_is_not_a_list_when_decoded_then_it_is_a_payload_error() {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[message_added(0)]);
    let mut done = message_done(0, Some("completed"), vec![]);
    done["item"]["content"] = json!("text");
    feed_err(&mut decoder, &done);
}

#[test]
fn given_a_done_part_with_a_non_text_text_when_decoded_then_it_is_a_payload_error() {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[message_added(0)]);
    let part = json!({ "type": "output_text", "text": 5 });
    feed_err(
        &mut decoder,
        &message_done(0, Some("completed"), vec![part]),
    );
}

#[test]
fn given_a_done_part_of_an_unknown_kind_when_decoded_then_it_is_a_payload_error() {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[message_added(0)]);
    let part = json!({ "type": "output_audio", "data": SENTINEL });
    let text = feed_err(
        &mut decoder,
        &message_done(0, Some("completed"), vec![part]),
    )
    .text()
    .to_owned();
    assert!(!text.contains(SENTINEL));
}

#[test]
fn given_a_wrong_typed_id_or_encrypted_content_when_a_reasoning_item_is_done_then_it_is_a_payload_error()
 {
    for field in ["id", "encrypted_content"] {
        let mut decoder = plain_decoder();
        flat(&mut decoder, &[reasoning_added(0)]);
        let mut done = reasoning_done(0, "rs_1", Some("E"), &["t"]);
        done["item"][field] = json!(12);
        feed_err(&mut decoder, &done);
    }
}

#[test]
fn given_a_summary_that_is_not_a_list_when_a_reasoning_item_is_done_then_it_is_a_payload_error() {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[reasoning_added(0)]);
    let mut done = reasoning_done(0, "rs_1", Some("E"), &[]);
    done["item"]["summary"] = json!("text");
    feed_err(&mut decoder, &done);
}

#[test]
fn given_a_function_call_added_with_wrong_typed_name_when_decoded_then_it_is_a_payload_error() {
    let mut decoder = plain_decoder();
    let mut added = call_added(0, "c", "n");
    added["item"]["name"] = json!(5);
    feed_err(&mut decoder, &added);
}

#[test]
fn given_an_event_whose_item_is_not_an_object_when_decoded_then_it_is_a_payload_error() {
    for item in [json!(null), json!("x"), json!([]), json!(3)] {
        for kind in ["response.output_item.added", "response.output_item.done"] {
            let mut decoder = plain_decoder();
            feed_err(
                &mut decoder,
                &json!({ "type": kind, "output_index": 0, "item": item }),
            );
        }
    }
}

#[test]
fn given_a_content_part_event_without_a_part_object_when_decoded_then_it_is_a_payload_error() {
    for part in [json!(null), json!("x"), json!([])] {
        let mut decoder = plain_decoder();
        flat(&mut decoder, &[message_added(0)]);
        let mut event = part_added(0, 0, output_part());
        event["part"] = part;
        feed_err(&mut decoder, &event);
    }
}

#[test]
fn given_every_payload_error_when_read_then_it_names_the_event_type_and_a_fixed_problem() {
    let mut decoder = plain_decoder();
    let error = feed_err(&mut decoder, &text_delta(9, 0, "x"));
    assert!(error.text().starts_with("response.output_text.delta: "));
    let error = feed_err(&mut decoder, &json!({ "type": "response.completed" }));
    assert!(error.text().starts_with("response.completed: "));
}
