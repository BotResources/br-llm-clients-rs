use std::collections::BTreeSet;
use std::path::Path;

use serde_json::Value;

use crate::test_recorded::recorded_json;
use crate::test_recordings::{REASONING_TOOL, RECORDED_ON, RECORDINGS};

fn listed_files() -> BTreeSet<String> {
    RECORDINGS
        .iter()
        .map(|recording| recording.file.to_owned())
        .collect()
}

fn files_on_disk() -> BTreeSet<String> {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/recordings");
    std::fs::read_dir(directory)
        .expect("the recordings directory")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

#[test]
fn given_the_recordings_when_listed_then_every_data_file_is_listed_once_with_the_recording_date() {
    assert_eq!(listed_files(), files_on_disk());
    assert_eq!(listed_files().len(), RECORDINGS.len());
    assert_eq!(RECORDED_ON, "2026-10-19");
    for recording in RECORDINGS {
        assert_eq!(recording.recorded_on, RECORDED_ON, "{}", recording.name);
        assert!(
            recording
                .status_line
                .starts_with(&recording.status.to_string()),
            "{}",
            recording.name
        );
        assert!(
            recording.request_id.starts_with("req_"),
            "{}",
            recording.name
        );
        assert!(
            !recording.body.contains("HTTP_STATUS="),
            "{}",
            recording.name
        );
    }
}

#[test]
fn given_the_recorded_streams_when_read_then_each_ends_with_one_terminal_event_and_no_error_event()
{
    for recording in RECORDINGS
        .iter()
        .filter(|recording| recording.status == 200)
    {
        let types: Vec<String> = recorded_json(recording)
            .iter()
            .map(|event| event["type"].as_str().expect("a type").to_owned())
            .collect();
        assert_eq!(types[0], "response.created", "{}", recording.name);
        let terminals = types
            .iter()
            .filter(|kind| {
                [
                    "response.completed",
                    "response.incomplete",
                    "response.failed",
                ]
                .contains(&kind.as_str())
            })
            .count();
        assert_eq!(terminals, 1, "{}", recording.name);
        assert!(
            types
                .last()
                .is_some_and(|kind| kind == "response.completed" || kind == "response.incomplete"),
            "{}",
            recording.name
        );
        assert!(
            !types.iter().any(|kind| kind == "error"),
            "{}",
            recording.name
        );
        assert!(recording.body.ends_with("\n\n"), "{}", recording.name);
    }
}

#[test]
fn given_the_recorded_streams_when_read_then_the_event_types_are_the_ones_the_decoder_lists_or_ignores()
 {
    let known: BTreeSet<&str> = [
        "response.created",
        "response.output_item.added",
        "response.content_part.added",
        "response.output_text.delta",
        "response.refusal.delta",
        "response.function_call_arguments.delta",
        "response.output_item.done",
        "response.completed",
        "response.incomplete",
    ]
    .into_iter()
    .collect();
    let ignored: BTreeSet<&str> = [
        "response.in_progress",
        "response.output_text.done",
        "response.refusal.done",
        "response.content_part.done",
        "response.function_call_arguments.done",
    ]
    .into_iter()
    .collect();
    let seen: BTreeSet<String> = RECORDINGS
        .iter()
        .filter(|recording| recording.status == 200)
        .flat_map(recorded_json)
        .map(|event| event["type"].as_str().expect("a type").to_owned())
        .collect();
    for kind in &seen {
        assert!(
            known.contains(kind.as_str()) || ignored.contains(kind.as_str()),
            "{kind}"
        );
    }
    for kind in ignored {
        assert!(seen.contains(kind), "{kind}");
    }
}

#[test]
fn given_the_recorded_reasoning_stream_when_read_then_the_call_arguments_are_streamed_and_the_function_call_is_added_after_the_reasoning_item()
 {
    let events = recorded_json(&REASONING_TOOL);
    let position = |kind: &str, item: &str| -> usize {
        events
            .iter()
            .position(|event| {
                event["type"] == kind && event["item"]["type"].as_str().unwrap_or(item) == item
            })
            .expect("the event")
    };
    assert!(
        position("response.output_item.done", "reasoning")
            < position("response.output_item.added", "function_call")
    );
    let deltas = events
        .iter()
        .filter(|event| event["type"] == "response.function_call_arguments.delta")
        .map(|event| event["delta"].as_str().expect("a delta"))
        .collect::<String>();
    let done: &Value = events
        .iter()
        .find(|event| event["type"] == "response.function_call_arguments.done")
        .expect("the done event");
    assert_eq!(deltas, done["arguments"].as_str().expect("arguments"));
}
