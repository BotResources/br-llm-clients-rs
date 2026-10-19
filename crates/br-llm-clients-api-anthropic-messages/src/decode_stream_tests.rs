use br_llm_clients_common_call::{ErrorInfo, ProviderBlock, ProviderEvent};
use br_llm_messages::{StopReason, Usage};

use crate::test_decode::*;
use crate::test_streams::*;

fn model() -> ProviderEvent {
    ProviderEvent::Model("claude-test-model".to_owned())
}

#[test]
fn given_the_documented_basic_stream_when_decoded_then_the_events_follow_the_table() {
    assert_eq!(
        decode_stream(false, BASIC),
        vec![
            model(),
            ProviderEvent::Usage(usage(25, 1)),
            start("0", ProviderBlock::Text),
            text_delta("0", "Hello"),
            text_delta("0", "!"),
            end_of("0"),
            ProviderEvent::Usage(usage(25, 15)),
            stop(StopReason::EndTurn),
            ProviderEvent::Complete,
        ]
    );
}

#[test]
fn given_the_documented_tool_use_stream_when_decoded_then_the_call_arguments_arrive_in_pieces() {
    assert_eq!(
        decode_stream(false, TOOL_USE),
        vec![
            model(),
            ProviderEvent::Usage(usage(472, 2)),
            start("0", ProviderBlock::Text),
            text_delta("0", "Okay"),
            text_delta("0", ","),
            text_delta("0", " let"),
            text_delta("0", "'s"),
            text_delta("0", " check"),
            text_delta("0", " the"),
            text_delta("0", " weather"),
            text_delta("0", " for"),
            text_delta("0", " San"),
            text_delta("0", " Francisco"),
            text_delta("0", ","),
            text_delta("0", " CA"),
            text_delta("0", ":"),
            end_of("0"),
            tool_start("1", "get_weather", "toolu_01T1x1fJ34qAmk2tNTrN7Up6"),
            args_delta("1", "{\"location\":"),
            args_delta("1", " \"San"),
            args_delta("1", " Francisc"),
            args_delta("1", "o,"),
            args_delta("1", " CA\"}"),
            end_of("1"),
            ProviderEvent::Usage(usage(472, 89)),
            stop(StopReason::AwaitingToolResults),
            ProviderEvent::Complete,
        ]
    );
}

#[test]
fn given_the_documented_thinking_stream_when_decoded_then_empty_start_fields_give_no_event_and_no_usage_is_given()
 {
    let events = decode_stream(false, THINKING);
    assert_eq!(
        events,
        vec![
            model(),
            start("0", ProviderBlock::Thinking),
            thinking_delta(
                "0",
                "I need to find the GCD of 1071 and 462 using the Euclidean algorithm.\n\n1071 = 2 × 462 + 147"
            ),
            thinking_delta("0", "\n462 = 3 × 147 + 21"),
            thinking_delta("0", "\n147 = 7 × 21 + 0"),
            thinking_delta("0", "\nThe remainder is 0, so GCD(1071, 462) = 21."),
            signature_delta(
                "0",
                "EqQBCgIYAhIM1gbcDa9GJwZA2b3hGgxBdjrkzLoky3dl1pkiMOYds..."
            ),
            end_of("0"),
            start("1", ProviderBlock::Text),
            text_delta(
                "1",
                "The greatest common divisor of 1071 and 462 is **21**."
            ),
            end_of("1"),
            stop(StopReason::EndTurn),
            ProviderEvent::Complete,
        ]
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, ProviderEvent::Usage(_)))
    );
}

#[test]
fn given_the_omitted_thinking_shape_when_decoded_then_the_empty_thinking_gives_only_the_signature()
{
    let events = decode_stream(false, crate::test_streams_constructed::OMITTED_THINKING);
    assert_eq!(
        &events[2..6],
        &[
            start("0", ProviderBlock::Thinking),
            signature_delta("0", "EqQBCgIYAhIM"),
            end_of("0"),
            start("1", ProviderBlock::Text),
        ]
    );
}

fn usages_of(stream: &str) -> Vec<Usage> {
    decode_stream(false, stream)
        .into_iter()
        .filter_map(|event| match event {
            ProviderEvent::Usage(usage) => Some(usage),
            _ => None,
        })
        .collect()
}

#[test]
fn given_the_documented_usage_lines_when_decoded_then_zero_cache_counts_are_some_zero_and_cumulative()
 {
    let zero = |input, output| Usage {
        input_tokens: input,
        output_tokens: output,
        cache_read_tokens: Some(0),
        cache_write_tokens: Some(0),
        thinking_tokens: None,
    };
    assert_eq!(
        usages_of(DOCUMENTED_USAGE),
        vec![zero(2679, 3), zero(10682, 510)]
    );
}

#[test]
fn given_a_constructed_stream_with_cache_counts_when_decoded_then_the_input_is_the_sum_of_three_fields()
 {
    assert_eq!(
        usages_of(crate::test_streams_constructed::CACHE_COUNTS),
        vec![
            Usage {
                input_tokens: 2679,
                output_tokens: 3,
                cache_read_tokens: Some(0),
                cache_write_tokens: Some(0),
                thinking_tokens: None,
            },
            Usage {
                input_tokens: 14802,
                output_tokens: 510,
                cache_read_tokens: Some(4000),
                cache_write_tokens: Some(120),
                thinking_tokens: None,
            },
        ]
    );
}

#[test]
fn given_the_documented_error_event_in_a_stream_when_decoded_then_it_gives_an_error_and_nothing_else()
 {
    let events = decode_stream(false, MID_STREAM_ERROR);
    let expected = ErrorInfo::new(
        r#"{"type": "error", "error": {"type": "overloaded_error", "message": "Overloaded"}}"#,
    )
    .with_code("overloaded_error");
    assert_eq!(events.last(), Some(&ProviderEvent::Error(expected)));
    assert_eq!(events.len(), 5);
}

#[test]
fn given_the_constructed_redacted_thinking_stream_when_decoded_then_the_data_is_in_the_start() {
    let events = decode_stream(false, crate::test_streams_constructed::REDACTED_THINKING);
    assert_eq!(
        &events[2..5],
        &[
            start("0", ProviderBlock::RedactedThinking),
            ProviderEvent::RedactedThinkingData {
                key: key("0"),
                data: "EmwKAhgBEgy3va3pzix/LafPsn4aDFIT2Xlxh0L5L8rLVyIwxtE3rAFBa8cr3qpP".to_owned(),
            },
            end_of("0"),
        ]
    );
}

#[test]
fn given_a_structured_reply_when_decoded_in_structured_mode_then_text_blocks_are_structured() {
    let events = decode_stream(true, crate::test_streams_constructed::STRUCTURED_REPLY);
    assert_eq!(
        &events[2..6],
        &[
            start("0", ProviderBlock::Structured),
            ProviderEvent::StructuredDelta {
                key: key("0"),
                json: "{\"value\":".to_owned()
            },
            ProviderEvent::StructuredDelta {
                key: key("0"),
                json: " \"forty-two\"}".to_owned(),
            },
            end_of("0"),
        ]
    );
}

#[test]
fn given_a_structured_reply_when_decoded_in_plain_mode_then_text_blocks_stay_text() {
    let events = decode_stream(false, crate::test_streams_constructed::STRUCTURED_REPLY);
    assert_eq!(events[2], start("0", ProviderBlock::Text));
}
