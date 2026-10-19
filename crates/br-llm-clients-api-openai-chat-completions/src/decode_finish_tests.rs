use br_llm_clients_common_call::{End, ProviderBlock, ProviderEvent};
use br_llm_messages::StopReason;
use serde_json::json;

use crate::test_support::{
    chunk, content_chunk, end, fails, feed, feed_all, finish_chunk, plain_decoder, run_chunks,
    start, text_delta, tool_chunk, usage_chunk,
};

fn stop_of(reason: &str, with_refusal: bool) -> StopReason {
    let mut chunks = Vec::new();
    if with_refusal {
        chunks.push(chunk(json!({"refusal": "no"}), None));
    }
    chunks.push(finish_chunk(reason));
    run_chunks(&chunks)
        .into_iter()
        .find_map(|event| match event {
            ProviderEvent::Stop(stop) => Some(stop),
            _ => None,
        })
        .expect("a stop")
}

fn other(text: &str) -> StopReason {
    StopReason::Other {
        reason: text.to_owned(),
    }
}

#[test]
fn given_each_finish_reason_when_decoded_then_the_stop_reason_follows_the_table() {
    let table = [
        ("stop", StopReason::EndTurn),
        ("length", StopReason::MaxTokens),
        ("tool_calls", StopReason::AwaitingToolResults),
        ("content_filter", StopReason::ContentFilter),
        ("function_call", other("function_call")),
        ("model_length", other("model_length")),
        ("STOP", other("STOP")),
        ("Stop ", other("Stop ")),
    ];
    for (reason, expected) in table {
        assert_eq!(stop_of(reason, false), expected, "{reason}");
    }
}

#[test]
fn given_a_refusal_and_an_end_turn_when_decoded_then_the_stop_reason_is_refusal() {
    assert_eq!(stop_of("stop", true), StopReason::Refusal);
}

#[test]
fn given_a_refusal_and_another_reason_when_decoded_then_the_reason_is_unchanged() {
    assert_eq!(stop_of("length", true), StopReason::MaxTokens);
    assert_eq!(stop_of("content_filter", true), StopReason::ContentFilter);
    assert_eq!(stop_of("tool_calls", true), StopReason::AwaitingToolResults);
    assert_eq!(stop_of("odd", true), other("odd"));
}

#[test]
fn given_a_blank_refusal_when_decoded_then_it_still_counts_as_a_refusal() {
    let events = run_chunks(&[chunk(json!({"refusal": "  "}), None), finish_chunk("stop")]);
    assert_eq!(
        events.last(),
        Some(&ProviderEvent::Stop(StopReason::Refusal))
    );
}

#[test]
fn given_an_empty_refusal_when_decoded_then_it_is_not_a_refusal() {
    let events = run_chunks(&[chunk(json!({"refusal": ""}), None), finish_chunk("stop")]);
    assert_eq!(events, [ProviderEvent::Stop(StopReason::EndTurn)]);
}

#[test]
fn given_a_finish_with_open_blocks_when_decoded_then_the_closing_events_come_before_the_stop() {
    let events = run_chunks(&[content_chunk("a"), finish_chunk("stop")]);
    assert_eq!(
        events,
        [
            start("text", ProviderBlock::Text),
            text_delta("text", "a"),
            end("text"),
            ProviderEvent::Stop(StopReason::EndTurn),
        ]
    );
}

#[test]
fn given_a_delta_and_a_finish_in_one_chunk_when_decoded_then_the_delta_is_read_first() {
    let both = chunk(json!({"content": "last words"}), Some("stop"));
    let events = run_chunks(&[both]);
    assert_eq!(
        events,
        [
            start("text", ProviderBlock::Text),
            text_delta("text", "last words"),
            end("text"),
            ProviderEvent::Stop(StopReason::EndTurn),
        ]
    );
}

#[test]
fn given_the_finish_reason_error_when_decoded_then_one_error_event_and_no_closing_or_stop() {
    let mut decoder = plain_decoder();
    feed(decoder.as_mut(), &content_chunk("partial")).expect("text");
    let events = feed(decoder.as_mut(), &finish_chunk("error")).expect("finish");
    assert_eq!(events.len(), 1);
    match events.first() {
        Some(ProviderEvent::Error(info)) => {
            assert_eq!(info.status(), None);
            assert_eq!(info.code(), None);
            assert_eq!(info.retry_after(), None);
            assert!(info.text().contains("error"));
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn given_the_finish_reason_error_with_a_nameless_held_call_when_decoded_then_the_provider_error_wins()
 {
    let mut decoder = plain_decoder();
    let nameless = tool_chunk(
        json!([{"index": 0, "id": "c", "function": {"arguments": "{}"}}]),
        None,
    );
    feed(decoder.as_mut(), &nameless).expect("held");
    let events = feed(decoder.as_mut(), &finish_chunk("error")).expect("finish");
    assert!(matches!(events.as_slice(), [ProviderEvent::Error(_)]));
}

#[test]
fn given_an_empty_or_null_finish_reason_when_decoded_then_there_is_no_finish() {
    let mut decoder = plain_decoder();
    for reason in [json!(""), json!(null)] {
        let data = chunk(json!({"content": "x"}), None).replace(
            "\"finish_reason\":null",
            &format!("\"finish_reason\":{reason}"),
        );
        let events = feed(decoder.as_mut(), &data).expect("chunk");
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, ProviderEvent::Stop(_)))
        );
    }
    assert!(feed(decoder.as_mut(), &finish_chunk("stop")).is_ok());
}

#[test]
fn given_a_second_finish_reason_when_decoded_then_it_is_a_payload_error_even_if_equal() {
    for second in ["stop", "length"] {
        let mut decoder = plain_decoder();
        feed(decoder.as_mut(), &finish_chunk("stop")).expect("first");
        assert!(
            feed(decoder.as_mut(), &finish_chunk(second)).is_err(),
            "{second}"
        );
    }
}

#[test]
fn given_output_after_the_finish_when_decoded_then_it_is_a_payload_error() {
    let deltas = [
        json!({"content": "late"}),
        json!({"refusal": "late"}),
        json!({"tool_calls": [{"index": 0, "function": {"name": "f"}}]}),
    ];
    for delta in deltas {
        let mut decoder = plain_decoder();
        feed(decoder.as_mut(), &finish_chunk("stop")).expect("finish");
        assert!(
            feed(decoder.as_mut(), &chunk(delta.clone(), None)).is_err(),
            "{delta}"
        );
    }
}

#[test]
fn given_empty_output_after_the_finish_when_decoded_then_nothing_is_given() {
    let mut decoder = plain_decoder();
    feed(decoder.as_mut(), &finish_chunk("stop")).expect("finish");
    let deltas = [
        json!({"content": ""}),
        json!({"content": null}),
        json!({"refusal": ""}),
        json!({"tool_calls": []}),
        json!({"tool_calls": null}),
        json!({}),
    ];
    for delta in deltas {
        let events = feed(decoder.as_mut(), &chunk(delta.clone(), None)).expect("quiet");
        assert!(
            events
                .iter()
                .all(|event| matches!(event, ProviderEvent::Model(_))),
            "{delta}"
        );
    }
}

#[test]
fn given_reasoning_text_after_the_finish_when_the_field_is_set_then_it_is_a_payload_error() {
    let configured = crate::test_support::options().with_reasoning_field("reasoning_content");
    let dialect = crate::test_support::dialect_for(crate::ReasoningLevel::Off, configured);
    let mut decoder = br_llm_clients_common_call::Dialect::decoder(
        &dialect,
        &br_llm_clients_core::OutputMode::Plain,
    );
    feed(decoder.as_mut(), &finish_chunk("stop")).expect("finish");
    assert!(
        feed(
            decoder.as_mut(),
            &chunk(json!({"reasoning_content": "late"}), None)
        )
        .is_err()
    );
    assert!(
        feed(
            decoder.as_mut(),
            &chunk(json!({"reasoning_content": ""}), None)
        )
        .is_ok()
    );
}

#[test]
fn given_a_usage_chunk_after_the_finish_when_decoded_then_the_usage_is_given_after_the_stop() {
    let events = run_chunks(&[
        content_chunk("a"),
        finish_chunk("stop"),
        usage_chunk(json!({"prompt_tokens": 3, "completion_tokens": 4})),
    ]);
    assert!(matches!(events.get(3), Some(ProviderEvent::Stop(_))));
    assert!(matches!(events.get(4), Some(ProviderEvent::Usage(_))));
}

#[test]
fn given_the_model_of_the_chunks_when_decoded_then_it_is_given_only_when_it_changes() {
    let mut decoder = plain_decoder();
    let named = |model: &str| chunk(json!({"content": "x"}), None).replace("gpt-4o", model);
    let given = |data: String, decoder: &mut dyn br_llm_clients_common_call::PayloadDecoder| {
        feed(decoder, &data)
            .expect("chunk")
            .into_iter()
            .filter_map(|event| match event {
                ProviderEvent::Model(model) => Some(model),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        given(named("gpt-4o-2024"), decoder.as_mut()),
        ["gpt-4o-2024"]
    );
    assert_eq!(
        given(named("gpt-4o-2024"), decoder.as_mut()),
        Vec::<String>::new()
    );
    assert_eq!(given(named(""), decoder.as_mut()), Vec::<String>::new());
    assert_eq!(given(named("gpt-5"), decoder.as_mut()), ["gpt-5"]);
    assert_eq!(
        given(named("gpt-4o-2024"), decoder.as_mut()),
        ["gpt-4o-2024"]
    );
}

#[test]
fn given_a_chunk_without_a_model_when_decoded_then_no_model_event_is_given() {
    let mut decoder = plain_decoder();
    let data = json!({"choices": [{"index": 0, "delta": {"content": "x"}}]}).to_string();
    let events = feed(decoder.as_mut(), &data).expect("chunk");
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, ProviderEvent::Model(_)))
    );
}

#[test]
fn given_the_end_of_the_body_after_a_finish_and_a_usage_when_ended_then_the_marker_is_still_missing()
 {
    let mut decoder = plain_decoder();
    let chunks = [
        content_chunk("a"),
        finish_chunk("stop"),
        usage_chunk(json!({"prompt_tokens": 1, "completion_tokens": 1})),
    ];
    let _ = feed_all(decoder.as_mut(), &chunks);
    assert_eq!(decoder.end(), Ok(End::MarkerMissing));
}

#[test]
fn given_a_finish_reason_of_the_wrong_type_when_decoded_then_the_field_is_named() {
    for value in [json!(1), json!(true), json!(["stop"]), json!({})] {
        let data = chunk(json!({}), None).replace(
            "\"finish_reason\":null",
            &format!("\"finish_reason\":{value}"),
        );
        assert!(fails(&data).text().contains("finish_reason"), "{value}");
    }
}
