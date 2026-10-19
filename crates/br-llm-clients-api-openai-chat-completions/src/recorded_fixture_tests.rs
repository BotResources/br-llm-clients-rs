use std::collections::BTreeSet;

use crate::test_recordings::{
    BAD_BODY, BAD_KEY, BAD_MODEL, LENGTH, REASONING, RECORDED_ON, RECORDINGS, REFUSAL, TEXT, TOOL,
};

#[test]
fn given_the_recordings_when_listed_then_each_carries_the_recording_date_and_a_known_status() {
    assert_eq!(RECORDED_ON, "2026-10-19");
    let statuses: Vec<(&str, u16)> = RECORDINGS
        .iter()
        .map(|recording| (recording.name, recording.status()))
        .collect();
    assert_eq!(
        statuses,
        [
            ("chat-text", 200),
            ("chat-tool", 200),
            ("chat-length", 200),
            ("chat-reasoning", 200),
            ("chat-refusal", 200),
            ("chat-badkey", 401),
            ("chat-badmodel", 404),
            ("chat-badbody", 400),
        ]
    );
    for recording in RECORDINGS {
        assert_eq!(recording.recorded_on, RECORDED_ON, "{}", recording.name);
        assert!(
            recording.request_id().starts_with("req_"),
            "{}",
            recording.name
        );
    }
}

#[test]
fn given_the_recordings_directory_when_listed_then_every_data_file_is_registered() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/recordings");
    let on_disk: BTreeSet<String> = std::fs::read_dir(directory)
        .expect("the directory")
        .map(|entry| {
            let path = entry.expect("an entry").path();
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .expect("a name")
                .to_owned()
        })
        .collect();
    let registered: BTreeSet<String> = RECORDINGS
        .iter()
        .map(|recording| recording.name.to_owned())
        .collect();
    assert_eq!(on_disk, registered);
}

#[test]
fn given_the_stream_recordings_when_read_then_each_is_whole_frames_ending_with_the_sentinel() {
    for recording in [TEXT, TOOL, LENGTH, REASONING, REFUSAL] {
        assert!(
            recording.body.ends_with("data: [DONE]\n\n"),
            "{}",
            recording.name
        );
        assert!(
            !recording.body.contains("HTTP_STATUS"),
            "{}",
            recording.name
        );
        assert_eq!(
            recording.header("content-type"),
            Some("text/event-stream; charset=utf-8"),
            "{}",
            recording.name
        );
        let frames = recording.frames();
        assert_eq!(frames.last().map(String::as_str), Some("[DONE]"));
        for frame in frames.iter().take(frames.len() - 1) {
            assert!(serde_json::from_str::<serde_json::Value>(frame).is_ok());
        }
    }
}

#[test]
fn given_the_error_recordings_when_read_then_the_body_has_the_recorded_length_and_no_curl_suffix() {
    for recording in [BAD_KEY, BAD_MODEL, BAD_BODY] {
        let length: usize = recording
            .header("content-length")
            .and_then(|value| value.parse().ok())
            .expect("a length");
        assert_eq!(recording.body.len(), length, "{}", recording.name);
        assert!(
            !recording.body.contains("HTTP_STATUS"),
            "{}",
            recording.name
        );
    }
    assert_eq!(BAD_KEY.header("content-type"), Some("text/plain"));
    assert_eq!(
        BAD_MODEL.header("content-type"),
        Some("application/json; charset=utf-8")
    );
}
