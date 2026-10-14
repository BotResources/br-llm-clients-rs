use crate::test_support::{decode_whole, frame, frames_of};
use crate::{DecodeError, Frame};

#[test]
fn given_several_data_lines_when_the_frame_closes_then_they_join_with_one_line_feed_and_no_trailing_one()
 {
    assert_eq!(
        frames_of(b"data: a\ndata: b\ndata: c\n\n"),
        vec![frame(None, "a\nb\nc")]
    );
}

#[test]
fn given_a_data_line_with_an_empty_value_when_the_frame_closes_then_the_frame_has_empty_text() {
    assert_eq!(frames_of(b"data:\n\n"), vec![frame(None, "")]);
    assert_eq!(frames_of(b"data: \n\n"), vec![frame(None, "")]);
}

#[test]
fn given_a_data_line_without_colon_when_the_frame_closes_then_the_frame_has_empty_text() {
    assert_eq!(frames_of(b"data\n\n"), vec![frame(None, "")]);
}

#[test]
fn given_empty_data_lines_around_a_value_when_joining_then_the_line_feeds_stay() {
    assert_eq!(
        frames_of(b"data\ndata: x\ndata:\n\n"),
        vec![frame(None, "\nx\n")]
    );
    assert_eq!(frames_of(b"data\ndata\n\n"), vec![frame(None, "\n")]);
}

#[test]
fn given_a_frame_without_data_line_when_it_closes_then_nothing_is_returned() {
    assert_eq!(frames_of(b"event: x\n\n"), vec![]);
    assert_eq!(frames_of(b"id: 1\n\n"), vec![]);
    assert_eq!(frames_of(b": c\n\n"), vec![]);
    assert_eq!(frames_of(b"\n\n\n"), vec![]);
}

#[test]
fn given_an_event_line_when_the_frame_closes_then_the_frame_carries_the_name() {
    assert_eq!(
        frames_of(b"event: add\ndata: 1\n\n"),
        vec![frame(Some("add"), "1")]
    );
}

#[test]
fn given_the_event_line_after_the_data_line_when_the_frame_closes_then_the_frame_carries_the_name()
{
    assert_eq!(
        frames_of(b"data: 1\nevent: add\n\n"),
        vec![frame(Some("add"), "1")]
    );
}

#[test]
fn given_two_event_lines_in_one_frame_when_it_closes_then_the_second_replaces_the_first() {
    assert_eq!(
        frames_of(b"event: a\nevent: b\ndata: x\n\n"),
        vec![frame(Some("b"), "x")]
    );
}

#[test]
fn given_an_empty_event_name_when_the_frame_closes_then_the_event_is_absent() {
    assert_eq!(frames_of(b"event:\ndata: x\n\n"), vec![frame(None, "x")]);
    assert_eq!(frames_of(b"event\ndata: x\n\n"), vec![frame(None, "x")]);
    assert_eq!(
        frames_of(b"event: a\nevent:\ndata: x\n\n"),
        vec![frame(None, "x")]
    );
}

#[test]
fn given_an_event_name_when_an_empty_line_closes_a_frame_then_the_next_frame_has_no_name() {
    assert_eq!(
        frames_of(b"event: a\ndata: 1\n\ndata: 2\n\n"),
        vec![frame(Some("a"), "1"), frame(None, "2")]
    );
}

#[test]
fn given_an_event_name_when_an_empty_line_closes_a_frame_without_data_then_the_name_is_still_reset()
{
    assert_eq!(
        frames_of(b"event: a\n\ndata: 2\n\n"),
        vec![frame(None, "2")]
    );
}

#[test]
fn given_an_event_name_when_an_empty_line_comes_with_no_frame_open_then_nothing_is_returned() {
    assert_eq!(frames_of(b"\n\ndata: x\n\n"), vec![frame(None, "x")]);
}

#[test]
fn given_consecutive_empty_lines_when_decoding_then_only_one_frame_comes_from_the_data() {
    assert_eq!(frames_of(b"data: x\n\n\n\n"), vec![frame(None, "x")]);
}

#[test]
fn given_one_space_after_the_colon_when_reading_the_value_then_exactly_one_is_removed() {
    assert_eq!(frames_of(b"data:x\n\n"), vec![frame(None, "x")]);
    assert_eq!(frames_of(b"data: x\n\n"), vec![frame(None, "x")]);
    assert_eq!(frames_of(b"data:  x\n\n"), vec![frame(None, " x")]);
    assert_eq!(frames_of(b"data:\tx\n\n"), vec![frame(None, "\tx")]);
}

#[test]
fn given_a_colon_inside_the_value_when_reading_then_only_the_first_colon_splits() {
    assert_eq!(frames_of(b"data: a:b: c\n\n"), vec![frame(None, "a:b: c")]);
    assert_eq!(
        frames_of(b"event: a:b\ndata: x\n\n"),
        vec![frame(Some("a:b"), "x")]
    );
}

#[test]
fn given_a_space_before_the_colon_when_reading_then_the_name_is_unknown() {
    assert_eq!(frames_of(b"data : x\n\n"), vec![]);
}

#[test]
fn given_field_names_in_other_case_when_reading_then_they_are_unknown_fields() {
    assert_eq!(frames_of(b"Data: x\nDATA: y\nEvent: z\n\n"), vec![]);
    assert_eq!(frames_of(b"Event: z\ndata: x\n\n"), vec![frame(None, "x")]);
}

#[test]
fn given_id_retry_and_unknown_fields_when_reading_then_they_are_ignored() {
    assert_eq!(
        frames_of(b"id: 1\nretry: 50\nunknown: v\nid\nretry\ndata: x\n\n"),
        vec![frame(None, "x")]
    );
}

#[test]
fn given_only_ignored_fields_when_an_empty_line_comes_then_no_frame_is_started() {
    let mut decoder = crate::Decoder::default();
    let mut frames = Vec::new();
    decoder
        .feed(b"id: 1\nretry: 2\nfoo: 3\n: c\n", &mut frames)
        .unwrap();
    assert_eq!(decoder.finish(), Ok(()));
    assert_eq!(frames, vec![]);
}

#[test]
fn given_comment_lines_between_fields_when_reading_then_they_are_ignored() {
    assert_eq!(
        frames_of(b": a\ndata: x\n: b\ndata: y\n: c\n\n"),
        vec![frame(None, "x\ny")]
    );
}

#[test]
fn given_a_comment_that_looks_like_a_field_when_reading_then_it_is_a_comment() {
    assert_eq!(frames_of(b":data: x\n\n"), vec![]);
    assert_eq!(frames_of(b":\n\n"), vec![]);
}

#[test]
fn given_a_leading_byte_order_mark_when_reading_then_it_is_skipped() {
    assert_eq!(
        frames_of(b"\xEF\xBB\xBFdata: x\n\n"),
        vec![frame(None, "x")]
    );
}

#[test]
fn given_a_byte_order_mark_before_a_colon_when_reading_then_the_line_is_a_comment() {
    assert_eq!(
        frames_of(b"\xEF\xBB\xBF: c\ndata: x\n\n"),
        vec![frame(None, "x")]
    );
}

#[test]
fn given_a_byte_order_mark_after_the_start_when_reading_then_it_is_not_skipped() {
    assert_eq!(
        frames_of(b"\n\xEF\xBB\xBFdata: x\n\ndata: y\n\n"),
        vec![frame(None, "y")]
    );
    assert_eq!(
        frames_of(b"data: a\n\xEF\xBB\xBFdata: b\n\n"),
        vec![frame(None, "a")]
    );
}

#[test]
fn given_a_byte_order_mark_inside_a_value_when_reading_then_it_stays_in_the_text() {
    assert_eq!(
        frames_of("data: a\u{feff}b\n\n".as_bytes()),
        vec![frame(None, "a\u{feff}b")]
    );
}

#[test]
fn given_two_byte_order_marks_at_the_start_when_reading_then_only_the_first_is_skipped() {
    assert_eq!(
        frames_of(b"\xEF\xBB\xBF\xEF\xBB\xBFdata: x\n\ndata: y\n\n"),
        vec![frame(None, "y")]
    );
}

#[test]
fn given_the_first_two_bytes_of_a_mark_then_other_text_when_reading_then_they_are_ordinary_bytes() {
    let outcome = decode_whole(b"\xEF\xBBdata: x\n\ndata: y\n\n");
    assert_eq!(outcome.frames, vec![frame(None, "y")]);
    assert_eq!(outcome.error, None);
}

#[test]
fn given_a_frame_with_data_and_a_later_open_one_when_the_input_ends_then_the_first_stands() {
    let outcome = decode_whole(b"data: a\n\ndata: b\n");
    assert_eq!(outcome.frames, vec![frame(None, "a")]);
    assert_eq!(outcome.error, Some(DecodeError::InputTruncated));
}

#[test]
fn given_an_empty_event_name_when_a_frame_is_built_then_the_event_is_absent() {
    assert_eq!(Frame::new(Some(String::new()), "d").event(), None);
}
