use br_llm_clients_common_call::{ProviderBlock, ProviderEvent};
use br_llm_messages::{ContentBlock, ReplayMaterial, ResponseDeltaKind, ResponseEnding, Usage};

use crate::test_call::*;
use crate::test_decode::*;
use crate::test_recorded::*;
use crate::test_thread::redacted_data as redacted_material;

const RECORDED_MODEL: &str = "claude-test-model";
const RECORDED_ID: &str = "msg_test_redacted_thinking";

fn index_key(position: usize) -> String {
    position.to_string()
}

fn counted(input: u64, output: u64) -> Usage {
    usage(input, output)
        .with_cache_read_input_tokens(0)
        .with_cache_creation_input_tokens(0)
}

fn assert_same_events(actual: &[ProviderEvent], expected: &[ProviderEvent]) {
    assert_eq!(actual.len(), expected.len(), "event count");
    for (position, (left, right)) in actual.iter().zip(expected).enumerate() {
        assert!(left == right, "event {position} differs");
    }
}

fn expected_events(stream: &str) -> Vec<ProviderEvent> {
    let mut events = vec![
        ProviderEvent::Model(RECORDED_MODEL.to_owned()),
        ProviderEvent::ResponseId(RECORDED_ID.to_owned()),
        ProviderEvent::Usage(counted(93, 4)),
    ];
    for (position, data) in recorded_redacted_data(stream).into_iter().enumerate() {
        let key = index_key(position);
        events.push(start(&key, ProviderBlock::RedactedThinking));
        events.push(redacted_data(&key, &data));
        events.push(end_of(&key));
    }
    let text_key = index_key(REDACTED_BLOCK_COUNT);
    events.push(start(&text_key, ProviderBlock::Text));
    for piece in recorded_text_pieces(stream) {
        events.push(text_delta(&text_key, &piece));
    }
    events.push(end_of(&text_key));
    events.push(ProviderEvent::Usage(
        counted(93, 99).with_thinking_tokens(41),
    ));
    events.push(stop(ResponseEnding::finished()));
    events.push(ProviderEvent::Complete);
    events
}

fn expected_redacted_materials(stream: &str) -> Vec<ReplayMaterial> {
    recorded_redacted_data(stream)
        .iter()
        .map(|data| redacted_material(data))
        .collect()
}

#[test]
fn given_the_real_stream_when_read_then_it_holds_the_recorded_shape() {
    let data = recorded_redacted_data(REDACTED_THINKING_REAL);
    assert_eq!(data.len(), REDACTED_BLOCK_COUNT);
    assert!(data.iter().all(|blob| !blob.is_empty()));
    assert!(
        recorded_text_pieces(REDACTED_THINKING_REAL)
            .concat()
            .starts_with("Hello! It looks like you've sent a system-level trigger string")
    );
    assert!(REDACTED_THINKING_REAL.ends_with("}\n\n"));
    assert!(!REDACTED_THINKING_REAL.contains("HTTP_STATUS"));
    assert!(REDACTED_THINKING_REAL.contains("\"index\":0    }"));
    assert!(!REDACTED_THINKING_REAL.contains("\"thinking\":"));
}

#[test]
fn given_the_real_stream_when_decoded_then_every_redacted_block_keeps_its_data_and_its_end() {
    let events = decode_stream(false, REDACTED_THINKING_REAL);
    assert_same_events(&events, &expected_events(REDACTED_THINKING_REAL));
}

#[test]
fn given_the_real_stream_when_fed_frame_by_frame_then_each_frame_gives_what_the_intent_says() {
    let mut decoder = decoder(false);
    let mut seen_data = Vec::new();
    for payload in payloads(REDACTED_THINKING_REAL) {
        let kind = payload.event().map(str::to_owned);
        let value: serde_json::Value = serde_json::from_str(payload.data()).expect("json");
        let events = decoder.payload(payload).expect("a readable payload");
        match kind.as_deref() {
            Some("ping") => assert!(events.is_empty()),
            Some("content_block_start")
                if value["content_block"]["type"] == "redacted_thinking" =>
            {
                let key = value["index"].to_string();
                assert_eq!(events.len(), 2);
                assert!(events[0] == start(&key, ProviderBlock::RedactedThinking));
                match &events[1] {
                    ProviderEvent::RedactedThinkingData {
                        key: data_key,
                        format,
                        data,
                    } => {
                        assert_eq!(data_key, &key);
                        assert_eq!(
                            format.as_str(),
                            "br-llm-clients-api-anthropic-messages/redacted-data"
                        );
                        seen_data.push(data.clone());
                    }
                    _ => panic!("not a redacted data event"),
                }
            }
            Some("content_block_stop") => {
                assert!(events == vec![end_of(&value["index"].to_string())]);
            }
            _ => {}
        }
    }
    assert!(seen_data == recorded_redacted_data(REDACTED_THINKING_REAL));
}

#[tokio::test]
async fn given_the_real_stream_when_run_then_the_response_holds_the_redacted_blocks_in_order_then_the_text()
 {
    let called = call_stream(&hello(), REDACTED_THINKING_REAL).await;
    let response = called.response();
    let blocks = response.blocks();
    let expected = expected_redacted_materials(REDACTED_THINKING_REAL);
    assert_eq!(blocks.len(), expected.len() + 1);
    for (block, material) in blocks.iter().zip(&expected) {
        match block {
            ContentBlock::RedactedThinking(redacted) => {
                assert!(redacted.material() == material);
            }
            _ => panic!("not a redacted thinking block"),
        }
    }
    match blocks.last() {
        Some(ContentBlock::Text(text)) => assert_eq!(
            text.text().as_str(),
            recorded_text_pieces(REDACTED_THINKING_REAL).concat()
        ),
        _ => panic!("the last block is not a text"),
    }
    assert_eq!(response.ending(), &ResponseEnding::finished());
    assert_eq!(
        response.model().map(|model| model.as_str()),
        Some(RECORDED_MODEL)
    );
    assert_eq!(
        response.provider_response_id().map(|id| id.as_str()),
        Some(RECORDED_ID)
    );
    let usage = response.usage().expect("a usage");
    assert_eq!(usage, &counted(93, 99).with_thinking_tokens(41));
    assert_eq!(called.outcome.report().usage(), Some(usage));
    assert_eq!(
        called.deltas.last().map(|delta| delta.kind()),
        Some(ResponseDeltaKind::ResponseCompleted)
    );
}

#[tokio::test]
async fn given_the_real_stream_split_in_two_writes_at_every_byte_when_run_then_the_outcome_never_changes()
 {
    let whole = call_stream(&hello(), REDACTED_THINKING_REAL).await;
    let server = crate::test_server::TestServer::start(|index| {
        crate::test_server::http(
            "200 OK",
            &[("content-type", "text/event-stream")],
            REDACTED_THINKING_REAL.as_bytes(),
        )
        .split_body_at(index)
    })
    .await;
    let dialect = std::sync::Arc::new(plain_dialect());
    let http = http_client();
    for offset in 0..REDACTED_THINKING_REAL.len() {
        let spec = spec_with(std::sync::Arc::clone(&dialect), &server, http.clone());
        let mut sink = crate::test_sink::RecordingSink::default();
        let outcome = br_llm_clients_common_call::run(&spec, &hello(), &mut sink).await;
        assert!(
            outcome.result() == whole.outcome.result(),
            "split at {offset}"
        );
        assert!(sink.deltas == whole.deltas, "split at {offset}");
    }
}
