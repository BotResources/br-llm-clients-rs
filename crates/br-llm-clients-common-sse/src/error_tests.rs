use crate::test_support::{decode_whole, feed_all, frame};
use crate::{DecodeError, Decoder};

#[test]
fn given_a_clean_stream_when_it_finishes_then_it_succeeds() {
    let mut decoder = Decoder::default();
    feed_all(&mut decoder, b"data: a\n\n").unwrap();
    assert_eq!(decoder.finish(), Ok(()));
}

#[test]
fn given_no_input_when_it_finishes_then_it_succeeds() {
    assert_eq!(Decoder::default().finish(), Ok(()));
}

#[test]
fn given_a_started_and_unclosed_frame_when_it_finishes_then_the_input_is_truncated() {
    assert_eq!(
        decode_whole(b"data: a\n").error,
        Some(DecodeError::InputTruncated)
    );
}

#[test]
fn given_a_frame_started_by_an_event_line_only_when_it_finishes_then_the_input_is_truncated() {
    assert_eq!(
        decode_whole(b"event: a\n").error,
        Some(DecodeError::InputTruncated)
    );
}

#[test]
fn given_an_unfinished_line_when_it_finishes_then_the_input_is_truncated() {
    assert_eq!(
        decode_whole(b"data: a\n\ndata: b").error,
        Some(DecodeError::InputTruncated)
    );
    assert_eq!(
        decode_whole(b"data: a\n\nid: 3").error,
        Some(DecodeError::InputTruncated)
    );
    assert_eq!(
        decode_whole(b": comment").error,
        Some(DecodeError::InputTruncated)
    );
}

#[test]
fn given_an_unfinished_line_after_a_bare_carriage_return_end_when_it_finishes_then_it_is_complete()
{
    assert_eq!(decode_whole(b"data: a\r\r").error, None);
}

#[test]
fn given_ignored_lines_only_when_it_finishes_then_the_input_is_complete() {
    assert_eq!(decode_whole(b": c\nid: 1\nretry: 2\nx: y\n").error, None);
}

#[test]
fn given_a_started_frame_closed_by_the_last_empty_line_when_it_finishes_then_it_succeeds() {
    let outcome = decode_whole(b"event: a\ndata: b\n\n");
    assert_eq!(outcome.frames, vec![frame(Some("a"), "b")]);
    assert_eq!(outcome.error, None);
}

#[test]
fn given_a_held_part_of_a_byte_order_mark_when_it_finishes_then_the_input_is_truncated() {
    assert_eq!(
        decode_whole(b"\xEF\xBB").error,
        Some(DecodeError::InputTruncated)
    );
}

#[test]
fn given_a_full_byte_order_mark_alone_when_it_finishes_then_the_input_is_complete() {
    assert_eq!(decode_whole(b"\xEF\xBB\xBF").error, None);
}

#[test]
fn given_a_truncated_input_when_it_fails_then_the_frames_already_returned_stand() {
    let outcome = decode_whole(b"data: a\n\ndata: b\n\ndata: c");
    assert_eq!(outcome.frames, vec![frame(None, "a"), frame(None, "b")]);
    assert_eq!(outcome.error, Some(DecodeError::InputTruncated));
}

#[test]
fn given_a_failed_finish_when_it_is_called_again_then_the_same_error_comes() {
    let mut decoder = Decoder::default();
    feed_all(&mut decoder, b"data: a").unwrap();
    assert_eq!(decoder.finish(), Err(DecodeError::InputTruncated));
    assert_eq!(decoder.finish(), Err(DecodeError::InputTruncated));
}

#[test]
fn given_a_finish_when_feeding_then_the_input_has_ended() {
    let mut decoder = Decoder::default();
    decoder.finish().unwrap();
    assert_eq!(
        feed_all(&mut decoder, b"data: a\n\n"),
        Err(DecodeError::InputEnded)
    );
    assert_eq!(feed_all(&mut decoder, b""), Err(DecodeError::InputEnded));
}

#[test]
fn given_a_failed_finish_when_feeding_then_the_same_error_comes_not_input_ended() {
    let mut decoder = Decoder::default();
    feed_all(&mut decoder, b"data: a").unwrap();
    assert_eq!(decoder.finish(), Err(DecodeError::InputTruncated));
    assert_eq!(
        feed_all(&mut decoder, b"\n\n"),
        Err(DecodeError::InputTruncated)
    );
    assert_eq!(
        feed_all(&mut decoder, b""),
        Err(DecodeError::InputTruncated)
    );
    assert_eq!(decoder.finish(), Err(DecodeError::InputTruncated));
}

#[test]
fn given_a_feed_error_then_a_finish_when_feeding_then_the_first_error_still_comes() {
    let mut decoder = Decoder::default();
    assert_eq!(
        feed_all(&mut decoder, b"data: \xFF\n"),
        Err(DecodeError::InvalidUtf8)
    );
    assert_eq!(decoder.finish(), Err(DecodeError::InvalidUtf8));
    assert_eq!(
        feed_all(&mut decoder, b"data: a\n\n"),
        Err(DecodeError::InvalidUtf8)
    );
    assert_eq!(decoder.finish(), Err(DecodeError::InvalidUtf8));
}

#[test]
fn given_a_finished_decoder_when_finish_is_called_again_then_it_succeeds_again() {
    let mut decoder = Decoder::default();
    assert_eq!(decoder.finish(), Ok(()));
    assert_eq!(decoder.finish(), Ok(()));
}

#[test]
fn given_a_feed_error_when_feeding_again_then_the_same_error_comes() {
    let mut decoder = Decoder::new(4).unwrap();
    let error = DecodeError::FrameTooLarge { limit: 4 };
    assert_eq!(feed_all(&mut decoder, b"data: abc"), Err(error.clone()));
    assert_eq!(feed_all(&mut decoder, b"\n\n"), Err(error.clone()));
    assert_eq!(feed_all(&mut decoder, b""), Err(error.clone()));
    assert_eq!(decoder.finish(), Err(error.clone()));
    assert_eq!(decoder.finish(), Err(error));
}

#[test]
fn given_an_utf8_error_when_feeding_or_finishing_then_the_same_error_repeats() {
    let mut decoder = Decoder::default();
    assert_eq!(
        feed_all(&mut decoder, b"data: \xFF\n"),
        Err(DecodeError::InvalidUtf8)
    );
    assert_eq!(
        feed_all(&mut decoder, b"data: ok\n\n"),
        Err(DecodeError::InvalidUtf8)
    );
    assert_eq!(decoder.finish(), Err(DecodeError::InvalidUtf8));
}

#[test]
fn given_a_feed_error_when_the_frames_list_is_given_then_no_frame_is_added_after_the_error() {
    let mut decoder = Decoder::default();
    let mut frames = Vec::new();
    assert!(
        decoder
            .feed(b"data: a\n\ndata: \xFF\n\ndata: z\n\n", &mut frames)
            .is_err()
    );
    assert_eq!(frames, vec![frame(None, "a")]);
    assert!(decoder.feed(b"data: later\n\n", &mut frames).is_err());
    assert_eq!(frames, vec![frame(None, "a")]);
}

#[test]
fn given_a_list_with_frames_when_feeding_then_new_frames_are_appended() {
    let mut decoder = Decoder::default();
    let mut frames = vec![frame(None, "old")];
    decoder.feed(b"data: new\n\n", &mut frames).unwrap();
    assert_eq!(frames, vec![frame(None, "old"), frame(None, "new")]);
}

#[test]
fn given_every_error_case_when_displayed_then_a_text_for_people_comes() {
    let cases = [
        DecodeError::InvalidLimit,
        DecodeError::FrameTooLarge { limit: 12 },
        DecodeError::InvalidUtf8,
        DecodeError::InputTruncated,
        DecodeError::InputEnded,
    ];
    let texts: Vec<String> = cases.iter().map(ToString::to_string).collect();
    for text in &texts {
        assert!(!text.is_empty());
        assert!(text.is_ascii());
    }
    for (index, text) in texts.iter().enumerate() {
        for other in texts.iter().skip(index + 1) {
            assert_ne!(text, other);
        }
    }
    assert!(texts[1].contains("12"));
}

#[test]
fn given_a_decoder_when_moved_across_threads_then_it_is_send() {
    fn is_send<T: Send>() {}
    is_send::<Decoder>();
    is_send::<crate::Frame>();
    is_send::<DecodeError>();
}

#[test]
fn given_a_decoder_holding_a_secret_when_it_is_printed_then_the_secret_never_appears() {
    let mut decoder = Decoder::default();
    feed_all(
        &mut decoder,
        b"event: SECRET-EVENT\ndata: SECRET-DATA\ndata: SECRET-PART",
    )
    .unwrap();
    for text in [format!("{decoder:?}"), format!("{decoder:#?}")] {
        assert!(!text.contains("SECRET"), "{text}");
        assert!(text.contains("limit"));
        assert!(text.contains("held"));
    }
}

#[test]
fn given_a_decoder_that_failed_with_a_secret_held_when_it_is_printed_then_the_secret_never_appears()
{
    let mut decoder = Decoder::new(40).unwrap();
    feed_all(&mut decoder, b"data: SECRET-DATA\n").unwrap();
    assert!(feed_all(&mut decoder, b"data: SECRET-TOO-LONG-TO-FIT-IN-FORTY-BYTES").is_err());
    assert!(!format!("{decoder:?}").contains("SECRET"));
}
