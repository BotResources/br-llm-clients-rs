use br_llm_clients_common_call::run;
use br_llm_clients_core::ClientError;
use br_llm_messages::{ModelResponse, ResponseDelta};

use crate::run_support::counted_spec_for;
use crate::test_recordings::{
    BAD_BODY, BAD_KEY, BAD_MODEL, LENGTH, REASONING, REFUSAL, Recording, TEXT, TOOL,
};
use crate::test_server::{Gate, TestServer, streamed};
use crate::test_sink::RecordingSink;
use crate::test_support::{dialect, hello};

#[derive(Debug, PartialEq)]
struct Summary {
    result: Result<ModelResponse, ClientError>,
    deltas: Vec<ResponseDelta>,
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

fn cuts(body: &str) -> Vec<usize> {
    let mut cuts = vec![0, body.len()];
    for (at, character) in body.char_indices() {
        let after = at + character.len_utf8();
        if character == '\n' {
            cuts.extend([after - 1, after, after + 1]);
        }
        cuts.extend(at + 1..after);
    }
    cuts.retain(|cut| *cut <= body.len());
    cuts.sort_unstable();
    cuts.dedup();
    cuts
}

async fn summaries(recording: Recording) -> Vec<Summary> {
    let offsets = cuts(recording.body);
    let served = offsets.clone();
    let server = TestServer::start(move |index| {
        cut_wire(recording, served.get(index).copied().unwrap_or(0))
    })
    .await;
    let mut found = Vec::new();
    for _ in &offsets {
        let spec = counted_spec_for(dialect(), &server);
        let mut sink = RecordingSink::default();
        let outcome = run(&spec, &hello(), &mut sink).await;
        found.push(Summary {
            request_id: outcome.report().request_id().map(str::to_owned),
            result: match outcome.result() {
                Ok(response) => Ok(response.clone()),
                Err(error) => Err(error.clone()),
            },
            deltas: sink.deltas,
        });
    }
    found
}

async fn assert_every_cut_gives_the_same_outcome(recording: Recording) {
    let found = summaries(recording).await;
    let offsets = cuts(recording.body);
    let reference = found.first().expect("the unsplit call");
    assert_eq!(
        reference.request_id.as_deref(),
        Some(recording.request_id())
    );
    assert_eq!(found.len(), offsets.len());
    for (cut, summary) in offsets.iter().zip(&found) {
        assert_eq!(summary, reference, "{} cut at {cut}", recording.name);
    }
}

#[test]
fn given_a_body_when_its_cuts_are_listed_then_they_hold_each_line_end_its_neighbours_and_the_inside_of_each_wide_character()
 {
    assert_eq!(cuts("ab\n\ncd"), [0, 2, 3, 4, 5, 6]);
    assert_eq!(cuts("a\u{e9}b"), [0, 2, 4]);
    assert_eq!(cuts("\u{20ac}\n"), [0, 1, 2, 3, 4]);
}

#[tokio::test]
async fn given_the_recorded_text_stream_when_cut_at_the_ends_the_line_ends_and_inside_wide_characters_then_the_outcome_is_the_same()
 {
    assert_every_cut_gives_the_same_outcome(TEXT).await;
}

#[tokio::test]
async fn given_the_recorded_tool_stream_when_cut_at_the_ends_the_line_ends_and_inside_wide_characters_then_the_outcome_is_the_same()
 {
    assert_every_cut_gives_the_same_outcome(TOOL).await;
}

#[tokio::test]
async fn given_the_recorded_length_stream_when_cut_at_the_ends_the_line_ends_and_inside_wide_characters_then_the_outcome_is_the_same()
 {
    assert_every_cut_gives_the_same_outcome(LENGTH).await;
}

#[tokio::test]
async fn given_the_recorded_reasoning_stream_when_cut_at_the_ends_the_line_ends_and_inside_wide_characters_then_the_outcome_is_the_same()
 {
    assert_every_cut_gives_the_same_outcome(REASONING).await;
}

#[tokio::test]
async fn given_the_recorded_refusal_stream_when_cut_at_the_ends_the_line_ends_and_inside_wide_characters_then_the_outcome_is_the_same()
 {
    assert_every_cut_gives_the_same_outcome(REFUSAL).await;
}

#[tokio::test]
async fn given_the_recorded_error_bodies_when_cut_at_the_ends_the_line_ends_and_inside_wide_characters_then_the_failure_is_the_same()
 {
    for recording in [BAD_KEY, BAD_MODEL, BAD_BODY] {
        assert_every_cut_gives_the_same_outcome(recording).await;
    }
}
