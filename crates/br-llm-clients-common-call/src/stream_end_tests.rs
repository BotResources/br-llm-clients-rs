use br_llm_clients_core::ClientError;

use crate::test_decoder::EndScript;
use crate::test_fixtures::user_request;
use crate::test_stream::{body, dialect_ending, drive, drive_one, frame, kinds};

fn good() -> Vec<u8> {
    body(&[&["start a text", "text a hi", "end a", "stop end_turn"]])
}

fn end_with(lines: &[&str]) -> EndScript {
    EndScript::Events(lines.iter().map(|line| (*line).to_owned()).collect())
}

#[tokio::test]
async fn given_a_body_that_ends_cleanly_when_read_then_end_is_called_once_and_the_call_is_complete()
{
    let dialect = dialect_ending(end_with(&[]));
    let driven = drive_one(&dialect, &user_request(), good()).await;
    assert!(driven.result.is_ok());
    assert_eq!(dialect.log.ends(), 1);
}

#[tokio::test]
async fn given_end_events_when_read_then_they_are_normalized_like_the_events_of_a_payload() {
    let dialect = dialect_ending(end_with(&["end a", "usage 1 2", "stop end_turn"]));
    let bytes = body(&[&["start a text", "text a hi"]]);
    let driven = drive_one(&dialect, &user_request(), bytes).await;
    assert!(driven.result.is_ok());
    assert_eq!(
        kinds(&driven.sink.events),
        vec![
            "start 0 Text",
            "text 0 \"hi\"",
            "end 0",
            "finish end_turn true"
        ]
    );
}

#[tokio::test]
async fn given_a_body_without_stop_when_it_ends_then_the_call_is_truncated_and_nothing_is_closed() {
    let dialect = dialect_ending(end_with(&[]));
    let bytes = body(&[&["start a text", "text a hi", "end a"]]);
    let driven = drive_one(&dialect, &user_request(), bytes).await;
    assert_eq!(driven.result, Err(ClientError::StreamTruncated));
    assert_eq!(
        kinds(&driven.sink.events),
        vec!["start 0 Text", "text 0 \"hi\"", "end 0"]
    );
}

#[tokio::test]
async fn given_a_body_with_no_frame_at_all_when_it_ends_then_the_call_is_truncated() {
    let dialect = dialect_ending(end_with(&[]));
    let driven = drive(&dialect, &user_request(), &[]).await;
    assert_eq!(driven.result, Err(ClientError::StreamTruncated));
    assert_eq!(dialect.log.ends(), 1);
    assert!(driven.sink.events.is_empty());
}

#[tokio::test]
async fn given_a_decoder_that_misses_its_end_marker_when_the_body_ends_then_the_call_is_truncated()
{
    let dialect = dialect_ending(EndScript::Missing);
    let driven = drive_one(&dialect, &user_request(), good()).await;
    assert_eq!(driven.result, Err(ClientError::StreamTruncated));
}

#[tokio::test]
async fn given_a_decoder_whose_end_fails_when_the_body_ends_then_the_reply_is_malformed_with_its_text()
 {
    let dialect = dialect_ending(EndScript::Failure("the end is broken".to_owned()));
    let driven = drive_one(&dialect, &user_request(), good()).await;
    assert_eq!(
        driven.result,
        Err(ClientError::MalformedResponse {
            detail: "the end is broken".to_owned()
        })
    );
}

#[tokio::test]
async fn given_end_events_with_an_error_when_the_body_ends_then_the_error_is_the_failure() {
    let dialect = dialect_ending(end_with(&["error - boom - gone"]));
    let driven = drive_one(&dialect, &user_request(), good()).await;
    assert_eq!(
        driven.result,
        Err(ClientError::ProviderFailed {
            status: None,
            code: Some("boom".to_owned()),
            body: "gone".to_owned()
        })
    );
}

#[tokio::test]
async fn given_a_body_that_ends_in_the_middle_of_a_frame_when_read_then_it_is_truncated_after_the_earlier_frames_were_processed()
 {
    let dialect = dialect_ending(end_with(&[]));
    let mut bytes = body(&[&["start a text", "text a hi"]]);
    bytes.extend_from_slice(b"data: stop end_turn\n");
    let driven = drive_one(&dialect, &user_request(), bytes).await;
    assert_eq!(driven.result, Err(ClientError::StreamTruncated));
    assert_eq!(dialect.log.ends(), 0);
    assert_eq!(
        kinds(&driven.sink.events),
        vec!["start 0 Text", "text 0 \"hi\""]
    );
}

#[tokio::test]
async fn given_a_complete_when_read_then_end_is_not_called_and_later_frames_are_not_decoded() {
    let dialect = dialect_ending(end_with(&[]));
    let mut bytes = body(&[&["start a text", "text a hi", "end a", "stop end_turn"]]);
    bytes.extend(frame(&["complete"]));
    bytes.extend(frame(&["start b text", "text b late"]));
    let driven = drive_one(&dialect, &user_request(), bytes).await;
    assert!(driven.result.is_ok());
    assert_eq!(dialect.log.ends(), 0);
    assert_eq!(dialect.log.payload_count(), 2);
    assert_eq!(driven.sink.events.len(), 4);
}

#[tokio::test]
async fn given_events_after_a_complete_in_the_same_list_when_read_then_they_are_dropped() {
    let dialect = dialect_ending(end_with(&[]));
    let bytes = body(&[&[
        "start a text",
        "text a hi",
        "end a",
        "stop end_turn",
        "complete",
        "start b text",
        "text b late",
        "end b",
        "error 500 - - boom",
    ]]);
    let driven = drive_one(&dialect, &user_request(), bytes).await;
    assert!(driven.result.is_ok());
    assert_eq!(
        kinds(&driven.sink.events),
        vec![
            "start 0 Text",
            "text 0 \"hi\"",
            "end 0",
            "finish end_turn false"
        ]
    );
}

#[tokio::test]
async fn given_a_complete_before_the_end_of_the_body_when_a_later_chunk_is_broken_then_it_does_not_count()
 {
    let dialect = dialect_ending(end_with(&[]));
    let mut first = good();
    first.extend(frame(&["complete"]));
    let chunks = [first, vec![0xFF; 16]];
    let driven = drive(&dialect, &user_request(), &chunks).await;
    assert!(driven.result.is_ok());
}

#[tokio::test]
async fn given_a_complete_when_the_same_chunk_ends_with_a_frame_too_large_then_the_decode_error_does_not_count()
 {
    let dialect = dialect_ending(end_with(&[]));
    let mut bytes = good();
    bytes.extend(frame(&["complete"]));
    bytes.extend(vec![b'x'; 9 * 1024 * 1024]);
    let driven = drive_one(&dialect, &user_request(), bytes).await;
    assert!(driven.result.is_ok());
}

#[tokio::test]
async fn given_frames_before_a_decode_error_when_read_then_they_are_processed_first_and_the_error_is_malformed()
 {
    let dialect = dialect_ending(end_with(&[]));
    let mut bytes = body(&[&["start a text", "text a hi"]]);
    bytes.extend(vec![b'x'; 9 * 1024 * 1024]);
    let driven = drive_one(&dialect, &user_request(), bytes).await;
    assert!(matches!(
        driven.result,
        Err(ClientError::MalformedResponse { .. })
    ));
    assert_eq!(
        kinds(&driven.sink.events),
        vec!["start 0 Text", "text 0 \"hi\""]
    );
    assert_eq!(dialect.log.ends(), 0);
}

#[tokio::test]
async fn given_a_complete_with_no_stop_when_read_then_the_call_is_truncated() {
    let dialect = dialect_ending(end_with(&[]));
    let bytes = body(&[&["start a text", "text a hi", "end a"], &["complete"]]);
    let driven = drive_one(&dialect, &user_request(), bytes).await;
    assert_eq!(driven.result, Err(ClientError::StreamTruncated));
    let only = drive_one(&dialect, &user_request(), frame(&["complete"])).await;
    assert_eq!(only.result, Err(ClientError::StreamTruncated));
}
