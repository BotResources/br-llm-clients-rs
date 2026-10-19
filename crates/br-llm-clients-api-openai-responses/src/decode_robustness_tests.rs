use br_llm_clients_common_call::{End, Payload, PayloadDecoder, ProviderEvent};
use serde_json::{Value, json};

use crate::test_decoder::{plain_decoder, structured_decoder};
use crate::test_events::{
    arguments_delta, call_added, call_done, completed, created, error_event, failed, in_progress,
    incomplete, message_added, message_done, output_done_part, output_part, part_added,
    reasoning_added, reasoning_done, reasoning_text_delta, refusal_delta, refusal_done_part,
    refusal_part, summary_delta, summary_part_added, text_delta, usage_object,
};

const SENTINEL: &str = "SENTINEL-prompt-content-9c1d";

fn full_stream() -> Vec<Value> {
    vec![
        created("gpt-5.1-2026-01-01"),
        in_progress(),
        reasoning_added(0),
        summary_part_added(0, 0),
        summary_delta(0, 0, "plan"),
        summary_delta(0, 1, "more"),
        reasoning_text_delta(0, 0, "raw"),
        reasoning_done(0, "rs_1", Some("ENC"), &["plan", "more"]),
        message_added(1),
        part_added(1, 0, output_part()),
        text_delta(1, 0, "Hello"),
        message_done(1, Some("completed"), vec![output_done_part("Hello")]),
        message_added(2),
        part_added(2, 0, refusal_part()),
        refusal_delta(2, 0, "No"),
        message_done(2, Some("completed"), vec![refusal_done_part("No")]),
        call_added(3, "call_1", "search"),
        arguments_delta(3, "{\"q\":1}"),
        call_done(3, "call_1", "search", "{\"q\":1}", Some("completed")),
        completed("gpt-5.1-2026-01-01", usage_object(5, 6)),
    ]
}

fn replacements() -> Vec<Value> {
    vec![
        json!(null),
        json!(true),
        json!(-1),
        json!(1.5),
        json!(u64::MAX),
        json!(SENTINEL),
        json!(""),
        json!([]),
        json!([SENTINEL]),
        json!({}),
        json!({ "type": SENTINEL }),
    ]
}

fn paths(value: &Value, prefix: &mut Vec<String>, out: &mut Vec<Vec<String>>) {
    out.push(prefix.clone());
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                prefix.push(key.clone());
                paths(child, prefix, out);
                prefix.pop();
            }
        }
        Value::Array(items) => {
            for (position, child) in items.iter().enumerate() {
                prefix.push(position.to_string());
                paths(child, prefix, out);
                prefix.pop();
            }
        }
        _ => {}
    }
}

fn replace_at(root: &mut Value, path: &[String], replacement: &Value) {
    let Some((first, rest)) = path.split_first() else {
        *root = replacement.clone();
        return;
    };
    let child = match root {
        Value::Object(map) => map.get_mut(first),
        Value::Array(items) => first.parse::<usize>().ok().and_then(|at| items.get_mut(at)),
        _ => None,
    };
    if let Some(child) = child {
        replace_at(child, rest, replacement);
    }
}

fn remove_at(root: &mut Value, path: &[String]) {
    let Some((last, parents)) = path.split_last() else {
        return;
    };
    let mut cursor = root;
    for step in parents {
        let next = match cursor {
            Value::Object(map) => map.get_mut(step),
            Value::Array(items) => step.parse::<usize>().ok().and_then(|at| items.get_mut(at)),
            _ => None,
        };
        match next {
            Some(next) => cursor = next,
            None => return,
        }
    }
    if let Value::Object(map) = cursor {
        map.remove(last);
    }
}

fn feed(
    decoder: &mut Box<dyn PayloadDecoder>,
    value: &Value,
) -> Result<Vec<ProviderEvent>, String> {
    decoder
        .payload(Payload::new(None, value.to_string()))
        .map_err(|error| error.text().to_owned())
}

fn primed(structured: bool, upto: usize) -> Box<dyn PayloadDecoder> {
    let mut decoder = if structured {
        structured_decoder()
    } else {
        plain_decoder()
    };
    for event in full_stream().iter().take(upto) {
        feed(&mut decoder, event).expect("the documented stream is readable");
    }
    decoder
}

#[test]
fn given_the_documented_stream_when_decoded_then_every_event_is_readable_and_it_completes() {
    let mut decoder = plain_decoder();
    let mut last = Vec::new();
    for event in full_stream() {
        last = feed(&mut decoder, &event).expect("readable");
    }
    assert_eq!(last.last(), Some(&ProviderEvent::Complete));
}

#[test]
fn given_any_field_of_any_event_replaced_by_a_value_of_the_wrong_shape_when_decoded_then_there_is_no_panic_and_no_quote()
 {
    let stream = full_stream();
    let mut checked = 0_usize;
    for (position, event) in stream.iter().enumerate() {
        let mut all = Vec::new();
        paths(event, &mut Vec::new(), &mut all);
        for path in &all {
            for replacement in replacements() {
                let mut mutant = event.clone();
                replace_at(&mut mutant, path, &replacement);
                for structured in [false, true] {
                    let mut decoder = primed(structured, position);
                    if let Err(text) = feed(&mut decoder, &mutant) {
                        assert!(!text.contains(SENTINEL), "{text}");
                    }
                    checked += 1;
                }
            }
        }
    }
    assert!(checked > 5000, "{checked}");
}

#[test]
fn given_any_field_of_any_event_removed_when_decoded_then_there_is_no_panic() {
    let stream = full_stream();
    for (position, event) in stream.iter().enumerate() {
        let mut all = Vec::new();
        paths(event, &mut Vec::new(), &mut all);
        for path in &all {
            let mut mutant = event.clone();
            remove_at(&mut mutant, path);
            let mut decoder = primed(false, position);
            let _ = feed(&mut decoder, &mutant);
        }
    }
}

#[test]
fn given_any_json_value_as_a_payload_when_decoded_then_there_is_no_panic() {
    let listed = [
        "response.created",
        "response.output_item.added",
        "response.content_part.added",
        "response.output_text.delta",
        "response.refusal.delta",
        "response.reasoning_summary_text.delta",
        "response.reasoning_text.delta",
        "response.function_call_arguments.delta",
        "response.output_item.done",
        "error",
        "response.completed",
        "response.incomplete",
        "response.failed",
    ];
    for kind in listed {
        for body in replacements() {
            let mut decoder = plain_decoder();
            let _ = feed(
                &mut decoder,
                &json!({ "type": kind, "response": body, "item": body, "part": body, "delta": body, "output_index": body }),
            );
            let _ = feed(&mut decoder, &json!({ "type": kind }));
        }
    }
    for value in replacements() {
        let mut decoder = plain_decoder();
        let _ = feed(&mut decoder, &value);
    }
}

#[test]
fn given_a_stream_cut_at_every_frame_boundary_when_ended_then_the_outcome_is_defined_and_the_marker_is_missing()
 {
    let stream = full_stream();
    for cut in 0..stream.len() {
        for structured in [false, true] {
            let mut decoder = if structured {
                structured_decoder()
            } else {
                plain_decoder()
            };
            for event in stream.iter().take(cut) {
                feed(&mut decoder, event).expect("readable");
            }
            assert_eq!(decoder.end().expect("defined"), End::MarkerMissing, "{cut}");
        }
    }
}

#[test]
fn given_the_failure_and_incomplete_endings_when_decoded_after_every_prefix_then_no_panic() {
    let endings = [
        failed("server_error", "boom"),
        incomplete(Some("max_output_tokens"), usage_object(1, 1)),
        incomplete(None, Value::Null),
        error_event(Some("rate_limit_exceeded"), "slow", None),
    ];
    let stream = full_stream();
    for ending in endings {
        for cut in 0..stream.len() {
            let mut decoder = primed(false, cut);
            let _ = feed(&mut decoder, &ending);
        }
    }
}
