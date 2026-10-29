use br_llm_clients_core::ClientError;
use serde_json::Value;

use crate::test_events::{
    call_added, call_done, completed, created, message_added, message_done, output_done_part,
    reasoning_added, reasoning_done, summary_delta, usage_object,
};
use crate::test_requests::{dialect, hello};
use crate::test_run::{call_with_wire, ok_wire, sse};
use br_llm_clients_core::ReasoningLevel;

fn compact_stream() -> Vec<Value> {
    vec![
        created("m"),
        reasoning_added(0),
        summary_delta(0, 0, "t"),
        reasoning_done(0, "rs_1", Some("E"), &["t"]),
        message_added(1),
        message_done(1, Some("completed"), vec![output_done_part("ok")]),
        call_added(2, "c", "n"),
        call_done(2, "c", "n", "{}", Some("completed")),
        completed("m", usage_object(1, 1)),
    ]
}

#[tokio::test]
async fn given_a_complete_stream_when_run_then_the_cut_at_full_length_is_the_only_success() {
    let full = sse(&compact_stream());
    let mut outcomes = Vec::new();
    for cut in frame_boundaries(&full).into_iter().chain([full.len()]) {
        let body = full.get(..cut).unwrap_or_default().to_vec();
        let called = call_with_wire(dialect(ReasoningLevel::Off), &hello(), move |_| {
            ok_wire(&body)
        })
        .await;
        outcomes.push((cut, called.outcome.into_parts().1));
    }
    for (cut, outcome) in outcomes {
        if cut == full.len() {
            assert!(outcome.is_ok(), "{cut}");
        } else {
            assert_eq!(
                outcome.expect_err("cut"),
                ClientError::StreamTruncated,
                "{cut}"
            );
        }
    }
}

fn frame_boundaries(body: &[u8]) -> Vec<usize> {
    body.windows(2)
        .enumerate()
        .filter(|(_, pair)| *pair == b"\n\n")
        .map(|(position, _)| position + 2)
        .collect()
}

#[tokio::test]
async fn given_every_byte_cut_of_a_stream_when_run_then_each_outcome_is_truncated_or_malformed() {
    let full = sse(&compact_stream());
    let length = full.len();
    let body = full.clone();
    let server = crate::test_server::TestServer::start(move |index| {
        let end = index.min(length.saturating_sub(1));
        ok_wire(body.get(..end).unwrap_or_default())
    })
    .await;
    for cut in 0..length {
        let spec = crate::test_run::spec_with(
            dialect(ReasoningLevel::Off),
            server.endpoint("/v1/responses"),
        );
        let mut sink = crate::test_run::RecordingSink::default();
        let outcome = br_llm_clients_common_call::run(&spec, &hello(), &mut sink).await;
        let failure = outcome
            .into_parts()
            .1
            .expect_err("a cut stream is never a reply");
        assert!(
            matches!(
                failure,
                ClientError::StreamTruncated | ClientError::MalformedResponse { .. }
            ),
            "{cut}: {failure:?}"
        );
    }
}
