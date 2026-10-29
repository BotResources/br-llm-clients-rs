use br_llm_clients_common_call::{ProviderBlock, ProviderEvent};
use br_llm_clients_core::OutputMode;
use br_llm_messages::{ResponseEnding, Usage};

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
    ProviderEvent::Usage(
        Usage::of_input_tokens(input)
            .with_output_tokens(output)
            .with_cache_read_input_tokens(0)
            .with_thinking_tokens(thinking),
    )
}

fn model(name: &str) -> ProviderEvent {
    ProviderEvent::Model(name.to_owned())
}

fn response_id(id: &str) -> ProviderEvent {
    ProviderEvent::ResponseId(id.to_owned())
}

#[test]
fn given_the_recorded_text_stream_when_decoded_then_the_events_are_the_ids_the_text_the_ending_the_usage_and_the_end()
 {
    let mut expected = vec![
        response_id("chatcmpl-EahhoXeIIdUZY6Yb0Ccw8zIzYJuea"),
        model("gpt-4.1-nano-2025-04-14"),
        start("text", ProviderBlock::Text),
    ];
    expected.extend(texts(
        "text",
        &["Hello", "!", " How", " are", " you", " today", "?"],
    ));
    expected.extend([
        end("text"),
        ProviderEvent::Stop(ResponseEnding::finished()),
        usage(13, 7, 0),
        ProviderEvent::Complete,
    ]);
    assert_eq!(decode(TEXT, &OutputMode::Plain), expected);
}

#[test]
fn given_the_recorded_length_stream_when_decoded_then_the_ending_is_output_exhaustion_after_the_text()
 {
    let mut expected = vec![
        response_id("chatcmpl-Eahhr3AAEYXtXnDIF8R3XISutDu1E"),
        model("gpt-4.1-nano-2025-04-14"),
        start("text", ProviderBlock::Text),
    ];
    expected.extend(texts("text", &["B", "ene", "ath", " the", " endless"]));
    expected.extend([
        end("text"),
        ProviderEvent::Stop(ResponseEnding::output_exhaustion()),
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
            response_id("chatcmpl-EahhprXaOW0Aw5te7JSXvI7bOTOOf"),
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
            ProviderEvent::Stop(ResponseEnding::tool_use()),
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
            response_id("chatcmpl-EahhsI8q2ReMFjmYEmShr56ieusVg"),
            model("gpt-5.4-mini-2026-03-17"),
            start("text", ProviderBlock::Text),
            text_delta("text", "391"),
            end("text"),
            ProviderEvent::Stop(ResponseEnding::finished()),
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
        response_id("chatcmpl-EahhsmJWZ9eY35X8XPZlMklA1CZKR"),
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
        ProviderEvent::Stop(ResponseEnding::refusal()),
        usage(50, 12, 0),
        ProviderEvent::Complete,
    ]);
    assert_eq!(decode(REFUSAL, &OutputMode::Plain), expected);
}
