use br_llm_clients_common_call::ProviderEvent;
use br_llm_clients_core::OutputMode;
use br_llm_messages::{StopReason, Usage};
use serde_json::{Value, json};

use crate::test_decoder::{
    arguments, end, model, start_call, start_text, start_thinking, stop, text,
};
use crate::test_recorded::{answer_mode, decode_recording, recorded_event};
use crate::test_recordings::{INCOMPLETE, REASONING_TOOL, REFUSAL, TEXT};

fn usage_with(input: u64, output: u64, thinking: u64) -> ProviderEvent {
    ProviderEvent::Usage(Usage {
        input_tokens: input,
        output_tokens: output,
        cache_read_tokens: Some(0),
        cache_write_tokens: Some(0),
        thinking_tokens: Some(thinking),
    })
}

fn texts(key: &str, deltas: &[&str]) -> Vec<ProviderEvent> {
    deltas.iter().map(|delta| text(key, delta)).collect()
}

fn text_stream(
    model_name: &str,
    deltas: &[&str],
    usage: ProviderEvent,
    reason: StopReason,
) -> Vec<ProviderEvent> {
    let mut events = vec![model(model_name), start_text("0.0")];
    events.extend(texts("0.0", deltas));
    events.extend([
        end("0.0"),
        usage,
        model(model_name),
        stop(reason),
        ProviderEvent::Complete,
    ]);
    events
}

#[test]
fn given_the_recorded_text_stream_when_decoded_then_the_events_are_the_intent_ones() {
    let expected = text_stream(
        "gpt-4.1-nano-2025-04-14",
        &["Hello", "!", " How", " are", " you", " today", "?"],
        usage_with(13, 8, 0),
        StopReason::EndTurn,
    );
    assert_eq!(decode_recording(&TEXT, &OutputMode::Plain), expected);
}

#[test]
fn given_the_recorded_incomplete_stream_when_decoded_then_the_text_is_closed_and_the_stop_is_max_tokens()
 {
    let expected = text_stream(
        "gpt-4.1-nano-2025-04-14",
        &[
            "B",
            "ene",
            "ath",
            " the",
            " bound",
            "less",
            " canvas",
            ",",
            " azure",
            " wide",
            ",",
            "  \n",
            "Where",
            " whispers",
            " of",
            " the",
        ],
        usage_with(15, 16, 0),
        StopReason::MaxTokens,
    );
    assert_eq!(decode_recording(&INCOMPLETE, &OutputMode::Plain), expected);
}

#[test]
fn given_the_recorded_refusal_stream_when_decoded_in_either_mode_then_the_refusal_is_a_text_block_with_the_stop_refusal()
 {
    let expected = text_stream(
        "gpt-4.1-mini-2025-04-14",
        &[
            "I'm", " sorry", ",", " but", " I", " can't", " assist", " with", " that", " request",
            ".",
        ],
        usage_with(44, 13, 0),
        StopReason::EndTurn,
    );
    let mut refused = expected.clone();
    let index = refused
        .iter()
        .position(|event| matches!(event, ProviderEvent::Stop(_)))
        .expect("a stop");
    refused[index] = stop(StopReason::Refusal);
    assert_eq!(decode_recording(&REFUSAL, &OutputMode::Plain), refused);
    assert_eq!(decode_recording(&REFUSAL, &answer_mode()), refused);
}

fn signature_of(events: &[ProviderEvent]) -> String {
    events
        .iter()
        .find_map(|event| match event {
            ProviderEvent::SignatureDelta { signature, .. } => Some(signature.clone()),
            _ => None,
        })
        .expect("a signature")
}

fn signature_event(signature: &str) -> ProviderEvent {
    ProviderEvent::SignatureDelta {
        key: "0".to_owned(),
        signature: signature.to_owned(),
    }
}

#[test]
fn given_the_recorded_reasoning_and_tool_stream_when_decoded_then_the_signature_is_the_done_item_and_the_call_comes_last()
 {
    let added = recorded_event(&REASONING_TOOL, "response.output_item.added", 0)["item"].clone();
    let done = recorded_event(&REASONING_TOOL, "response.output_item.done", 0)["item"].clone();
    assert_ne!(added["encrypted_content"], done["encrypted_content"]);
    assert_eq!(added["id"], done["id"]);
    let actual = decode_recording(&REASONING_TOOL, &OutputMode::Plain);
    let carried = signature_of(&actual);
    assert_eq!(
        serde_json::from_str::<Value>(&carried).expect("the envelope is JSON"),
        json!({ "id": done["id"], "encrypted_content": done["encrypted_content"] })
    );
    let expected = vec![
        model("gpt-5.4-mini-2026-03-17"),
        start_thinking("0"),
        signature_event(&carried),
        end("0"),
        start_call("1", "get_weather", Some("call_ncwwZsUdlv8xT4aac97RgMXa")),
        arguments("1", "{\"city\":\"Paris\"}"),
        end("1"),
        usage_with(54, 30, 10),
        model("gpt-5.4-mini-2026-03-17"),
        stop(StopReason::EndTurn),
        ProviderEvent::Complete,
    ];
    assert_eq!(actual, expected);
}

#[test]
fn given_the_recorded_reasoning_item_when_read_then_it_holds_no_summary_even_though_a_summary_was_asked()
 {
    let done = recorded_event(&REASONING_TOOL, "response.output_item.done", 0)["item"].clone();
    assert_eq!(done["summary"], json!([]));
    assert_eq!(done["content"], json!([]));
    let kinds = decode_recording(&REASONING_TOOL, &OutputMode::Plain);
    assert!(
        !kinds
            .iter()
            .any(|event| matches!(event, ProviderEvent::ThinkingDelta { .. }))
    );
}
