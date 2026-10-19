use br_llm_clients_common_call::{Payload, PayloadDecoder};
use serde_json::{Value, json};

use crate::test_decode::*;
use crate::test_recordings::RECORDINGS;
use crate::test_streams::*;
use crate::test_streams_constructed::*;

const EVENT_TYPES: [&str; 8] = [
    "message_start",
    "content_block_start",
    "content_block_delta",
    "content_block_stop",
    "message_delta",
    "message_stop",
    "ping",
    "error",
];

fn wrong_values() -> Vec<Value> {
    vec![
        Value::Null,
        json!(true),
        json!(-1),
        json!(1.5),
        json!(u64::MAX),
        json!(""),
        json!("x"),
        json!([]),
        json!([1, "a"]),
        json!({}),
        json!({"type": 5}),
        json!({"nested": {"deep": [null]}}),
    ]
}

fn fields_of(kind: &str) -> Vec<&'static str> {
    match kind {
        "message_start" => vec!["message", "usage", "model"],
        "content_block_start" => vec![
            "index",
            "content_block",
            "type",
            "text",
            "data",
            "id",
            "name",
            "input",
        ],
        "content_block_delta" => vec![
            "index",
            "delta",
            "text",
            "partial_json",
            "thinking",
            "signature",
        ],
        "content_block_stop" => vec!["index"],
        "message_delta" => vec![
            "delta",
            "usage",
            "stop_reason",
            "input_tokens",
            "output_tokens",
        ],
        _ => vec!["error", "message"],
    }
}

fn primed() -> Box<dyn PayloadDecoder> {
    let mut decoder = decoder(false);
    for payload in payloads(TOOL_USE).into_iter().take(4) {
        decoder.payload(payload).expect("prelude");
    }
    decoder
}

#[test]
fn given_any_field_of_any_event_set_to_a_wrong_value_when_decoded_then_there_is_never_a_panic() {
    for kind in EVENT_TYPES {
        for field in fields_of(kind) {
            for wrong in wrong_values() {
                let mut object = json!({"type": kind});
                object[field] = wrong.clone();
                let mut fresh = decoder(false);
                let _ = fresh.payload(data(&object.to_string()));
                let mut primed = primed();
                let _ = primed.payload(data(&object.to_string()));
                let _ = primed.end();
            }
        }
    }
}

#[test]
fn given_any_whole_value_as_the_data_when_decoded_then_there_is_never_a_panic() {
    for wrong in wrong_values() {
        let mut fresh = decoder(true);
        let _ = fresh.payload(Payload::new(None, wrong.to_string()));
        let _ = fresh.payload(Payload::new(Some("error".to_owned()), wrong.to_string()));
    }
}

#[test]
fn given_a_wrong_nested_value_in_a_block_start_when_decoded_then_it_is_a_payload_error_not_a_panic()
{
    for wrong in wrong_values() {
        let mut session = Session::started();
        let payload = json!({"type": "content_block_start", "index": 0, "content_block": wrong});
        let result = session.feed(&payload.to_string());
        if !wrong.is_object() {
            assert!(result.is_err(), "{wrong}");
        }
    }
}

#[test]
fn given_every_documented_stream_when_cut_at_every_frame_boundary_then_the_outcome_is_defined() {
    for (_, stream, _) in RECORDINGS {
        let all = payloads(stream);
        for cut in 0..=all.len() {
            let mut decoder = decoder(false);
            let mut outcome = Ok(());
            for payload in all.iter().take(cut).cloned() {
                if let Err(error) = decoder.payload(payload) {
                    outcome = Err(error);
                    break;
                }
            }
            if outcome.is_ok() {
                assert_eq!(
                    decoder.end(),
                    Ok(br_llm_clients_common_call::End::MarkerMissing)
                );
            }
        }
    }
}

#[test]
fn given_a_full_documented_stream_when_decoded_then_exactly_one_complete_ends_it() {
    for stream in [
        BASIC,
        TOOL_USE,
        THINKING,
        DOCUMENTED_USAGE,
        CACHE_COUNTS,
        EMPTY_TOOL_CALL,
        REDACTED_THINKING,
    ] {
        let events = decode_stream(false, stream);
        let completes = events
            .iter()
            .filter(|event| matches!(event, br_llm_clients_common_call::ProviderEvent::Complete))
            .count();
        assert_eq!(completes, 1);
        assert_eq!(
            events.last(),
            Some(&br_llm_clients_common_call::ProviderEvent::Complete)
        );
    }
}
