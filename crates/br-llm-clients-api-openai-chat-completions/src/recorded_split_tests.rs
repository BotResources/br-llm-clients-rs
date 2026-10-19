use br_llm_clients_common_call::run;
use br_llm_clients_core::ClientError;
use br_llm_messages::{Step, StreamEvent};

use crate::run_support::spec_for;
use crate::test_recordings::{
    BAD_BODY, BAD_KEY, BAD_MODEL, LENGTH, REASONING, REFUSAL, Recording, TEXT, TOOL,
};
use crate::test_server::{Gate, TestServer, streamed};
use crate::test_sink::RecordingSink;
use crate::test_support::{dialect, hello};

#[derive(Debug, PartialEq)]
struct Summary {
    result: Result<Step, ClientError>,
    events: Vec<StreamEvent>,
    request_id: Option<String>,
}

fn pause() -> Gate {
    Box::pin(async {
        for _ in 0..20 {
            tokio::task::yield_now().await;
        }
    })
}

fn cut_wire(recording: Recording, cut: usize) -> crate::test_server::Wire {
    let status = format!("{} Recorded", recording.status());
    let headers: Vec<(&str, &str)> = recording
        .headers()
        .into_iter()
        .filter(|(name, _)| !name.eq_ignore_ascii_case("content-length"))
        .collect();
    let bytes = recording.body.as_bytes();
    let (first, rest) = bytes.split_at(cut.min(bytes.len()));
    streamed(&status, &headers, first, vec![(pause(), rest.to_vec())])
}

async fn summaries(recording: Recording) -> Vec<Summary> {
    let server = TestServer::start(move |index| cut_wire(recording, index)).await;
    let spec = spec_for(dialect(), &server);
    let mut found = Vec::new();
    for _ in 0..=recording.body.len() {
        let mut sink = RecordingSink::default();
        let outcome = run(&spec, &hello(), &mut sink).await;
        found.push(Summary {
            request_id: outcome.report().request_id().map(str::to_owned),
            result: match outcome.result() {
                Ok(step) => Ok(step.clone()),
                Err(error) => Err(error.clone()),
            },
            events: sink.events,
        });
    }
    found
}

async fn assert_every_cut_gives_the_same_outcome(recording: Recording) {
    let found = summaries(recording).await;
    let reference = found.first().expect("the unsplit call");
    assert_eq!(
        reference.request_id.as_deref(),
        Some(recording.request_id())
    );
    assert_eq!(found.len(), recording.body.len() + 1);
    for (cut, summary) in found.iter().enumerate() {
        assert_eq!(summary, reference, "{} cut at {cut}", recording.name);
    }
}

#[tokio::test]
async fn given_the_recorded_text_stream_when_cut_at_every_byte_then_the_outcome_is_the_same() {
    assert_every_cut_gives_the_same_outcome(TEXT).await;
}

#[tokio::test]
async fn given_the_recorded_tool_stream_when_cut_at_every_byte_then_the_outcome_is_the_same() {
    assert_every_cut_gives_the_same_outcome(TOOL).await;
}

#[tokio::test]
async fn given_the_recorded_length_stream_when_cut_at_every_byte_then_the_outcome_is_the_same() {
    assert_every_cut_gives_the_same_outcome(LENGTH).await;
}

#[tokio::test]
async fn given_the_recorded_reasoning_stream_when_cut_at_every_byte_then_the_outcome_is_the_same() {
    assert_every_cut_gives_the_same_outcome(REASONING).await;
}

#[tokio::test]
async fn given_the_recorded_refusal_stream_when_cut_at_every_byte_then_the_outcome_is_the_same() {
    assert_every_cut_gives_the_same_outcome(REFUSAL).await;
}

#[tokio::test]
async fn given_the_recorded_error_bodies_when_cut_at_every_byte_then_the_failure_is_the_same() {
    for recording in [BAD_KEY, BAD_MODEL, BAD_BODY] {
        assert_every_cut_gives_the_same_outcome(recording).await;
    }
}
