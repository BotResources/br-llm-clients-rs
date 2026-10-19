use br_llm_clients_core::ClientError;

use crate::test_call::*;
use crate::test_server::{TestServer, http};
use crate::test_streams::*;
use crate::test_streams_constructed::*;

async fn outcomes_for_every_cut(
    stream: &'static str,
    offsets: Vec<usize>,
) -> Vec<(usize, Option<ClientError>)> {
    let cuts = offsets.clone();
    let server = TestServer::start(move |index| {
        let end = cuts.get(index).copied().unwrap_or(0);
        http(
            "200 OK",
            &[],
            stream.as_bytes().get(..end).unwrap_or_default(),
        )
    })
    .await;
    let spec = spec(plain_dialect(), &server);
    let mut results = Vec::new();
    for offset in offsets {
        let mut sink = crate::test_sink::RecordingSink::default();
        let outcome = br_llm_clients_common_call::run(&spec, &hello(), &mut sink).await;
        results.push((offset, outcome.result().err().cloned()));
    }
    results
}

#[tokio::test]
async fn given_the_basic_stream_cut_at_every_byte_when_run_then_every_proper_prefix_is_truncated() {
    let offsets: Vec<usize> = (0..=BASIC.len()).collect();
    let results = outcomes_for_every_cut(BASIC, offsets).await;
    for (offset, error) in results {
        if offset == BASIC.len() {
            assert_eq!(error, None);
        } else {
            assert_eq!(error, Some(ClientError::StreamTruncated), "cut at {offset}");
        }
    }
}

#[tokio::test]
async fn given_the_tool_stream_cut_at_every_frame_boundary_when_run_then_every_proper_prefix_is_truncated()
 {
    let mut offsets = vec![0];
    let mut position = 0;
    for frame in TOOL_USE.split_inclusive("\n\n") {
        position += frame.len();
        offsets.push(position);
    }
    let results = outcomes_for_every_cut(TOOL_USE, offsets).await;
    let last = results.len() - 1;
    for (position, (offset, error)) in results.into_iter().enumerate() {
        if position == last {
            assert_eq!(error, None, "cut at {offset}");
        } else {
            assert_eq!(error, Some(ClientError::StreamTruncated), "cut at {offset}");
        }
    }
}

#[tokio::test]
async fn given_a_stream_cut_before_message_stop_when_run_then_it_is_truncated_whatever_it_held() {
    for stream in [BASIC, THINKING, EMPTY_TOOL_CALL, REDACTED_THINKING] {
        let cut = stream.find("event: message_stop").expect("a message_stop");
        let called = call_stream(&hello(), &stream[..cut]).await;
        assert_eq!(called.error(), &ClientError::StreamTruncated);
    }
}

#[tokio::test]
async fn given_a_message_stop_with_no_stop_reason_when_run_then_the_call_is_truncated() {
    let stream = BASIC.replace(
        r#"{"type": "message_delta", "delta": {"stop_reason": "end_turn", "stop_sequence":null}, "usage": {"output_tokens": 15}}"#,
        r#"{"type": "message_delta", "delta": {"stop_reason": null}, "usage": {"output_tokens": 15}}"#,
    );
    let called = call_stream(&hello(), &stream).await;
    assert_eq!(called.error(), &ClientError::StreamTruncated);
}

#[tokio::test]
async fn given_a_body_with_no_frame_at_all_when_run_then_the_call_is_truncated() {
    let called = call_stream(&hello(), "").await;
    assert_eq!(called.error(), &ClientError::StreamTruncated);
}

#[tokio::test]
async fn given_a_thinking_stream_cut_before_its_message_delta_when_run_then_it_is_truncated() {
    let cut = THINKING
        .find("event: message_delta")
        .expect("a message_delta");
    let called = call_stream(&hello(), &THINKING[..cut]).await;
    assert_eq!(called.error(), &ClientError::StreamTruncated);
}
