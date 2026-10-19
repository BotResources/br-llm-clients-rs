use crate::test_streams::*;
use crate::test_streams_constructed::*;

pub(crate) const RECORDINGS: [(&str, &str, &str); 12] = [
    ("BASIC", BASIC, RECORDED_ON),
    ("TOOL_USE", TOOL_USE, RECORDED_ON),
    ("THINKING", THINKING, RECORDED_ON),
    ("DOCUMENTED_USAGE", DOCUMENTED_USAGE, RECORDED_ON),
    ("MID_STREAM_ERROR", MID_STREAM_ERROR, RECORDED_ON),
    ("EMPTY_TOOL_CALL", EMPTY_TOOL_CALL, RECORDED_ON),
    ("OMITTED_THINKING", OMITTED_THINKING, RECORDED_ON),
    ("REDACTED_THINKING", REDACTED_THINKING, RECORDED_ON),
    ("STRUCTURED_REPLY", STRUCTURED_REPLY, RECORDED_ON),
    ("REFUSAL", REFUSAL, RECORDED_ON),
    (
        "MAX_TOKENS_IN_TOOL_CALL",
        MAX_TOKENS_IN_TOOL_CALL,
        RECORDED_ON,
    ),
    ("CACHE_COUNTS", CACHE_COUNTS, RECORDED_ON),
];

fn declared_names(source: &str) -> Vec<String> {
    source
        .lines()
        .filter_map(|line| line.strip_prefix("pub(crate) const "))
        .filter(|rest| rest.contains(": &str = r#\""))
        .filter_map(|rest| rest.split(':').next())
        .map(str::to_owned)
        .collect()
}

#[test]
fn given_the_stream_fixtures_when_listed_then_every_one_carries_the_recording_date() {
    let mut declared = declared_names(include_str!("test_streams.rs"));
    declared.extend(declared_names(include_str!("test_streams_constructed.rs")));
    let mut listed: Vec<String> = RECORDINGS
        .iter()
        .map(|(name, _, _)| (*name).to_owned())
        .collect();
    declared.sort();
    listed.sort();
    assert_eq!(listed, declared);
    assert!(!RECORDED_ON.is_empty());
    for (name, stream, date) in RECORDINGS {
        assert_eq!(date, RECORDED_ON, "{name}");
        assert!(stream.starts_with("event: message_start\n"), "{name}");
    }
}
