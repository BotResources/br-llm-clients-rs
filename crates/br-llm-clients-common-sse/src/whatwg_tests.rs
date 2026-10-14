use crate::DecodeError;
use crate::test_support::{decode_whole, frame, frames_of};

#[test]
fn given_the_stock_ticker_example_when_decoding_then_the_data_lines_join() {
    assert_eq!(
        frames_of(b"data: YHOO\ndata: +2\ndata: 10\n\n"),
        vec![frame(None, "YHOO\n+2\n10")]
    );
}

#[test]
fn given_the_comment_id_and_spacing_example_when_decoding_then_three_frames_come() {
    let stream = b": test stream\n\ndata: first event\nid: 1\n\ndata:second event\nid\n\ndata:  third event\n\n";
    assert_eq!(
        frames_of(stream),
        vec![
            frame(None, "first event"),
            frame(None, "second event"),
            frame(None, " third event"),
        ]
    );
}

#[test]
fn given_the_empty_data_example_when_decoding_then_the_empty_and_the_single_feed_frames_come() {
    assert_eq!(
        frames_of(b"data\n\ndata\ndata\n\n"),
        vec![frame(None, ""), frame(None, "\n")]
    );
}

#[test]
fn given_the_empty_data_example_without_a_closing_line_when_finishing_then_the_input_is_truncated()
{
    let outcome = decode_whole(b"data\n\ndata\ndata\n\ndata:");
    assert_eq!(outcome.frames, vec![frame(None, ""), frame(None, "\n")]);
    assert_eq!(outcome.error, Some(DecodeError::InputTruncated));
}

#[test]
fn given_the_leading_space_example_when_decoding_then_both_frames_carry_the_same_text() {
    assert_eq!(
        frames_of(b"data:test\n\ndata: test\n\n"),
        vec![frame(None, "test"), frame(None, "test")]
    );
}

#[test]
fn given_the_named_events_example_when_decoding_then_each_frame_carries_its_name() {
    let stream =
        b"event: add\ndata: 73857293\n\nevent: remove\ndata: 2153\n\nevent: add\ndata: 113411\n\n";
    assert_eq!(
        frames_of(stream),
        vec![
            frame(Some("add"), "73857293"),
            frame(Some("remove"), "2153"),
            frame(Some("add"), "113411"),
        ]
    );
}

#[test]
fn given_an_openai_style_stream_when_decoding_then_the_end_marker_is_an_ordinary_frame() {
    let stream = b"data: {\"id\":1,\"delta\":\"Hel\"}\n\ndata: {\"id\":1,\"delta\":\"lo\"}\n\ndata: [DONE]\n\n";
    assert_eq!(
        frames_of(stream),
        vec![
            frame(None, "{\"id\":1,\"delta\":\"Hel\"}"),
            frame(None, "{\"id\":1,\"delta\":\"lo\"}"),
            frame(None, "[DONE]"),
        ]
    );
}

#[test]
fn given_an_anthropic_style_stream_when_decoding_then_names_and_pings_are_kept_apart() {
    let stream = b"event: message_start\ndata: {\"type\":\"message_start\"}\n\nevent: ping\ndata: {\"type\": \"ping\"}\n\n: keep-alive\n\nevent: message_stop\ndata: {\"type\":\"message_stop\"}\n\n";
    assert_eq!(
        frames_of(stream),
        vec![
            frame(Some("message_start"), "{\"type\":\"message_start\"}"),
            frame(Some("ping"), "{\"type\": \"ping\"}"),
            frame(Some("message_stop"), "{\"type\":\"message_stop\"}"),
        ]
    );
}

#[test]
fn given_a_crlf_stream_with_a_byte_order_mark_when_decoding_then_the_frames_are_clean() {
    let stream = b"\xEF\xBB\xBFevent: a\r\ndata: 1\r\n\r\n: ping\r\n\r\ndata: 2\r\n\r\n";
    assert_eq!(
        frames_of(stream),
        vec![frame(Some("a"), "1"), frame(None, "2")]
    );
}

#[test]
fn given_an_input_that_ends_inside_a_frame_when_decoding_then_the_partial_event_is_an_error_not_a_drop()
 {
    let outcome = decode_whole(b"data: whole\n\ndata: part");
    assert_eq!(outcome.frames, vec![frame(None, "whole")]);
    assert_eq!(outcome.error, Some(DecodeError::InputTruncated));
}

#[test]
fn given_an_invalid_byte_in_a_kept_line_when_decoding_then_it_is_an_error_not_a_replacement() {
    let outcome = decode_whole(b"data: a\xFFb\n\n");
    assert_eq!(outcome.error, Some(DecodeError::InvalidUtf8));
}
