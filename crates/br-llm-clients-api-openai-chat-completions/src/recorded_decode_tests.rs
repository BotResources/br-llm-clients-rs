use br_llm_clients_common_call::{ProviderBlock, ProviderEvent};
use br_llm_clients_core::OutputMode;
use br_llm_messages::{StopReason, Usage};

use crate::test_recordings::{LENGTH, REASONING, REFUSAL, Recording, TEXT, TOOL};
use crate::test_support::{decoder_for, end, feed_all, start, text_delta};

fn decode(recording: Recording, output: &OutputMode) -> Vec<ProviderEvent> {
    let mut decoder = decoder_for(output);
    feed_all(decoder.as_mut(), &recording.frames())
}

fn texts(key: &str, pieces: &[&str]) -> Vec<ProviderEvent> {
    pieces.iter().map(|piece| text_delta(key, piece)).collect()
}

fn usage(input: u64, output: u64, thinking: u64) -> ProviderEvent {
    ProviderEvent::Usage(Usage {
        input_tokens: input,
        output_tokens: output,
        cache_read_tokens: Some(0),
        cache_write_tokens: None,
        thinking_tokens: Some(thinking),
    })
}

fn model(name: &str) -> ProviderEvent {
    ProviderEvent::Model(name.to_owned())
}

#[test]
fn given_the_recorded_text_stream_when_decoded_then_the_events_are_the_text_the_stop_the_usage_and_the_end()
 {
    let mut expected = vec![
        model("gpt-4.1-nano-2025-04-14"),
        start("text", ProviderBlock::Text),
    ];
    expected.extend(texts(
        "text",
        &["Hello", "!", " How", " are", " you", " today", "?"],
    ));
    expected.extend([
        end("text"),
        ProviderEvent::Stop(StopReason::EndTurn),
        usage(13, 7, 0),
        ProviderEvent::Complete,
    ]);
    assert_eq!(decode(TEXT, &OutputMode::Plain), expected);
}

#[test]
fn given_the_recorded_length_stream_when_decoded_then_the_stop_is_max_tokens_after_the_text() {
    let mut expected = vec![
        model("gpt-4.1-nano-2025-04-14"),
        start("text", ProviderBlock::Text),
    ];
    expected.extend(texts("text", &["B", "ene", "ath", " the", " endless"]));
    expected.extend([
        end("text"),
        ProviderEvent::Stop(StopReason::MaxTokens),
        usage(15, 5, 0),
        ProviderEvent::Complete,
    ]);
    assert_eq!(decode(LENGTH, &OutputMode::Plain), expected);
}

#[test]
fn given_the_recorded_tool_stream_when_decoded_then_the_call_comes_whole_at_the_finish() {
    let events = decode(TOOL, &OutputMode::Plain);
    assert_eq!(
        events,
        [
            model("gpt-4.1-nano-2025-04-14"),
            start(
                "0",
                ProviderBlock::ToolCall {
                    name: "get_weather".to_owned(),
                    id: Some("call_ZJ4D7BK3ZGpJHd4vnCNAj22l".to_owned()),
                }
            ),
            ProviderEvent::ToolArgumentsDelta {
                key: "0".to_owned(),
                json: "{\"city\": \"Paris\"}".to_owned(),
            },
            end("0"),
            ProviderEvent::Stop(StopReason::AwaitingToolResults),
            usage(55, 30, 0),
            ProviderEvent::Complete,
        ]
    );
}

#[test]
fn given_the_recorded_reasoning_stream_when_decoded_then_there_is_no_thinking_and_the_reasoning_tokens_are_counted()
 {
    let events = decode(REASONING, &OutputMode::Plain);
    assert_eq!(
        events,
        [
            model("gpt-5.4-mini-2026-03-17"),
            start("text", ProviderBlock::Text),
            text_delta("text", "391"),
            end("text"),
            ProviderEvent::Stop(StopReason::EndTurn),
            usage(20, 21, 11),
            ProviderEvent::Complete,
        ]
    );
    assert!(events.iter().all(|event| !matches!(
        event,
        ProviderEvent::ThinkingDelta { .. }
            | ProviderEvent::BlockStart {
                block: ProviderBlock::Thinking,
                ..
            }
    )));
}

#[test]
fn given_the_recorded_refusal_stream_when_decoded_then_the_null_content_and_empty_refusal_give_nothing_and_the_refusal_is_a_text_block()
 {
    let mut expected = vec![
        model("gpt-4.1-mini-2025-04-14"),
        start("refusal", ProviderBlock::Text),
    ];
    expected.extend(texts(
        "refusal",
        &[
            "I'm", " sorry", ",", " but", " I", " can't", " assist", " with", " that", " request",
            ".",
        ],
    ));
    expected.extend([
        end("refusal"),
        ProviderEvent::Stop(StopReason::Refusal),
        usage(50, 12, 0),
        ProviderEvent::Complete,
    ]);
    assert_eq!(decode(REFUSAL, &OutputMode::Plain), expected);
}
