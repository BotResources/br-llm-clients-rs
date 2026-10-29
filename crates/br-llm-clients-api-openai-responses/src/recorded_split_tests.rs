use br_llm_clients_core::{ClientRequest, OutputMode, ReasoningLevel};

use crate::test_recorded::{
    answer_request, decode_all, payloads_of, run_recording, run_split, weather_request,
};
use crate::test_recordings::{INCOMPLETE, REASONING_TOOL, REFUSAL, Recording, TEXT};
use crate::test_requests::{dialect, hello};

fn every_byte_boundary_gives_the_same_events(recording: &Recording, mode: &OutputMode) {
    let bytes = recording.body.as_bytes();
    let subject = dialect(ReasoningLevel::Off);
    let reference = decode_all(&subject, mode, payloads_of(&[bytes]));
    assert!(!reference.is_empty());
    for cut in 1..bytes.len() {
        let (head, tail) = bytes.split_at(cut);
        let split = decode_all(&subject, mode, payloads_of(&[head, tail]));
        assert_eq!(split, reference, "{} cut at {cut}", recording.name);
    }
}

#[test]
fn given_each_recorded_stream_when_cut_at_every_byte_boundary_then_the_decoded_events_are_unchanged()
 {
    every_byte_boundary_gives_the_same_events(&TEXT, &OutputMode::Plain);
    every_byte_boundary_gives_the_same_events(&REASONING_TOOL, &OutputMode::Plain);
    every_byte_boundary_gives_the_same_events(&INCOMPLETE, &OutputMode::Plain);
    every_byte_boundary_gives_the_same_events(&REFUSAL, &OutputMode::Plain);
}

#[test]
fn given_a_recorded_stream_when_sent_one_byte_at_a_time_then_the_decoded_events_are_unchanged() {
    let subject = dialect(ReasoningLevel::Off);
    for recording in [TEXT, REASONING_TOOL, INCOMPLETE, REFUSAL] {
        let bytes = recording.body.as_bytes();
        let reference = decode_all(&subject, &OutputMode::Plain, payloads_of(&[bytes]));
        let singles: Vec<&[u8]> = bytes.chunks(1).collect();
        let split = decode_all(&subject, &OutputMode::Plain, payloads_of(&singles));
        assert_eq!(split, reference, "{}", recording.name);
    }
}

fn interesting_cuts(recording: &Recording) -> Vec<usize> {
    let bytes = recording.body.as_bytes();
    let mut cuts = vec![1, bytes.len() / 2, bytes.len().saturating_sub(1)];
    for (position, window) in bytes.windows(2).enumerate() {
        if window == b"\n\n" {
            let boundary = position.saturating_add(2);
            cuts.extend([position, position.saturating_add(1), boundary]);
            cuts.push(boundary.saturating_add(3));
        }
    }
    if let Some(cipher) = recording.body.find("encrypted_content") {
        cuts.extend([cipher.saturating_add(40), cipher.saturating_add(400)]);
    }
    cuts.retain(|cut| *cut > 0 && *cut < bytes.len());
    cuts.sort_unstable();
    cuts.dedup();
    cuts
}

async fn split_calls_match_the_whole_call(
    recording: &Recording,
    level: ReasoningLevel,
    request: &ClientRequest,
) {
    let whole = run_recording(recording, level, request).await;
    let (whole_report, whole_result) = whole.outcome.into_parts();
    let cuts = interesting_cuts(recording);
    assert!(cuts.len() > 20);
    for cut in cuts {
        let split = run_split(recording, level, request, cut).await;
        assert_eq!(
            split.sink.deltas, whole.sink.deltas,
            "{} cut {cut}",
            recording.name
        );
        let (report, result) = split.outcome.into_parts();
        assert_eq!(result, whole_result, "{} cut {cut}", recording.name);
        assert_eq!(
            report.usage(),
            whole_report.usage(),
            "{} cut {cut}",
            recording.name
        );
        assert_eq!(
            report.model(),
            whole_report.model(),
            "{} cut {cut}",
            recording.name
        );
        assert_eq!(
            report.request_id(),
            whole_report.request_id(),
            "{} cut {cut}",
            recording.name
        );
    }
}

#[tokio::test]
async fn given_the_recorded_streams_when_the_body_arrives_in_two_writes_then_run_gives_the_same_outcome_at_every_frame_boundary()
 {
    split_calls_match_the_whole_call(&TEXT, ReasoningLevel::Off, &hello()).await;
    split_calls_match_the_whole_call(&INCOMPLETE, ReasoningLevel::Off, &hello()).await;
    split_calls_match_the_whole_call(&REFUSAL, ReasoningLevel::Off, &answer_request()).await;
    split_calls_match_the_whole_call(&REASONING_TOOL, ReasoningLevel::Low, &weather_request())
        .await;
}
