use crate::test_support::{decode_with, feed_all, frame};
use crate::{DEFAULT_LIMIT, DecodeError, Decoder};

#[test]
fn given_a_limit_of_zero_when_creating_then_it_is_refused() {
    assert_eq!(Decoder::new(0).unwrap_err(), DecodeError::InvalidLimit);
}

#[test]
fn given_no_limit_when_creating_the_default_then_it_is_8_mib() {
    assert_eq!(DEFAULT_LIMIT, 8 * 1024 * 1024);
    assert_eq!(DEFAULT_LIMIT, 8_388_608);
    assert_eq!(Decoder::default().limit(), 8_388_608);
}

#[test]
fn given_an_unfinished_line_exactly_at_the_limit_when_feeding_then_it_is_allowed() {
    let mut decoder = Decoder::new(10).unwrap();
    assert_eq!(feed_all(&mut decoder, b"data: abcd"), Ok(vec![]));
    assert_eq!(
        feed_all(&mut decoder, b"\n\n"),
        Ok(vec![frame(None, "abcd")])
    );
}

#[test]
fn given_an_unfinished_line_one_byte_over_the_limit_when_feeding_then_it_fails_with_the_limit() {
    let mut decoder = Decoder::new(10).unwrap();
    assert_eq!(
        feed_all(&mut decoder, b"data: abcde"),
        Err(DecodeError::FrameTooLarge { limit: 10 })
    );
}

#[test]
fn given_a_line_that_passes_the_limit_inside_one_chunk_when_feeding_then_the_error_comes_before_the_line_end()
 {
    let mut decoder = Decoder::new(10).unwrap();
    assert_eq!(
        feed_all(&mut decoder, b"data: a-very-long-line-without-end"),
        Err(DecodeError::FrameTooLarge { limit: 10 })
    );
}

#[test]
fn given_bytes_fed_one_at_a_time_when_the_limit_is_passed_then_the_error_comes_on_that_byte() {
    let mut decoder = Decoder::new(5).unwrap();
    for byte in b"abcde" {
        assert_eq!(feed_all(&mut decoder, &[*byte]), Ok(vec![]));
    }
    assert_eq!(
        feed_all(&mut decoder, b"f"),
        Err(DecodeError::FrameTooLarge { limit: 5 })
    );
}

#[test]
fn given_a_comment_line_longer_than_the_limit_when_feeding_then_it_fails_because_the_line_is_unfinished()
 {
    let mut decoder = Decoder::new(8).unwrap();
    assert_eq!(
        feed_all(&mut decoder, b": a comment longer than eight"),
        Err(DecodeError::FrameTooLarge { limit: 8 })
    );
}

#[test]
fn given_a_comment_line_exactly_at_the_limit_when_it_completes_then_it_holds_nothing() {
    let mut decoder = Decoder::new(8).unwrap();
    assert_eq!(feed_all(&mut decoder, b": abcdef\n"), Ok(vec![]));
    assert_eq!(feed_all(&mut decoder, b": abcdef\n"), Ok(vec![]));
    assert_eq!(decoder.finish(), Ok(()));
}

#[test]
fn given_a_long_run_of_keep_alive_comments_when_feeding_then_the_limit_is_never_reached() {
    let mut decoder = Decoder::new(10).unwrap();
    for _ in 0..10_000 {
        assert_eq!(feed_all(&mut decoder, b": ping\n"), Ok(vec![]));
    }
    assert_eq!(
        feed_all(&mut decoder, b"data: x\n\n"),
        Ok(vec![frame(None, "x")])
    );
}

#[test]
fn given_a_long_run_of_ignored_fields_when_feeding_then_the_limit_is_never_reached() {
    let mut decoder = Decoder::new(10).unwrap();
    for _ in 0..10_000 {
        assert_eq!(
            feed_all(&mut decoder, b"id: 12\nretry: 9\nx: y\n"),
            Ok(vec![])
        );
    }
    assert_eq!(decoder.finish(), Ok(()));
}

#[test]
fn given_a_long_run_of_empty_lines_when_feeding_then_the_limit_is_never_reached() {
    let mut decoder = Decoder::new(1).unwrap();
    for _ in 0..1_000 {
        assert_eq!(feed_all(&mut decoder, b"\n\r\r\n"), Ok(vec![]));
    }
}

#[test]
fn given_an_event_name_held_by_the_open_frame_when_a_line_arrives_then_both_count() {
    let mut decoder = Decoder::new(12).unwrap();
    assert_eq!(feed_all(&mut decoder, b"event:abc\n"), Ok(vec![]));
    assert_eq!(feed_all(&mut decoder, b"id:123456"), Ok(vec![]));
    assert_eq!(
        feed_all(&mut decoder, b"7"),
        Err(DecodeError::FrameTooLarge { limit: 12 })
    );
}

#[test]
fn given_an_event_name_and_a_line_exactly_at_the_limit_when_feeding_then_it_is_allowed() {
    let mut decoder = Decoder::new(12).unwrap();
    assert_eq!(feed_all(&mut decoder, b"event:abc\n"), Ok(vec![]));
    assert_eq!(feed_all(&mut decoder, b"id:123456"), Ok(vec![]));
    assert_eq!(feed_all(&mut decoder, b"\n\n"), Ok(vec![]));
}

#[test]
fn given_data_text_held_by_the_open_frame_when_more_lines_arrive_then_the_text_counts() {
    let mut decoder = Decoder::new(20).unwrap();
    assert_eq!(feed_all(&mut decoder, b"data:aaaaaaaa\n"), Ok(vec![]));
    assert_eq!(feed_all(&mut decoder, b"data:bbb\n"), Ok(vec![]));
    assert_eq!(feed_all(&mut decoder, b"data:cc"), Ok(vec![]));
    assert_eq!(feed_all(&mut decoder, b"\n"), Ok(vec![]));
    assert_eq!(feed_all(&mut decoder, b"data:"), Ok(vec![]));
    assert_eq!(
        feed_all(&mut decoder, b"d"),
        Err(DecodeError::FrameTooLarge { limit: 20 })
    );
}

#[test]
fn given_data_text_and_a_line_exactly_at_the_limit_when_feeding_then_it_is_allowed() {
    let mut decoder = Decoder::new(20).unwrap();
    assert_eq!(feed_all(&mut decoder, b"data:aaaaaaaa\n"), Ok(vec![]));
    assert_eq!(feed_all(&mut decoder, b"data:bbb\n"), Ok(vec![]));
    assert_eq!(feed_all(&mut decoder, b"data:cc\n"), Ok(vec![]));
    assert_eq!(feed_all(&mut decoder, b"data:"), Ok(vec![]));
    assert_eq!(
        feed_all(&mut decoder, b"\n\n"),
        Ok(vec![frame(None, "aaaaaaaa\nbbb\ncc\n")])
    );
}

#[test]
fn given_a_frame_closed_when_the_next_one_starts_then_the_held_bytes_count_from_zero() {
    let mut decoder = Decoder::new(12).unwrap();
    for _ in 0..50 {
        assert_eq!(
            feed_all(&mut decoder, b"event:abc\ndata:wxyz\n\n"),
            Ok(vec![frame(Some("abc"), "wxyz")])
        );
    }
}

#[test]
fn given_a_data_line_larger_than_the_limit_in_one_frame_when_decoding_then_the_stream_fails() {
    let outcome = decode_with(8, &[b"data: 0123456789\n\n"]);
    assert_eq!(outcome.frames, vec![]);
    assert_eq!(outcome.error, Some(DecodeError::FrameTooLarge { limit: 8 }));
}

#[test]
fn given_frames_before_the_oversize_line_when_decoding_then_they_are_kept() {
    let outcome = decode_with(12, &[b"data: a\n\ndata: b\n\ndata: 0123456789\n\n"]);
    assert_eq!(outcome.frames, vec![frame(None, "a"), frame(None, "b")]);
    assert_eq!(
        outcome.error,
        Some(DecodeError::FrameTooLarge { limit: 12 })
    );
}

#[test]
fn given_a_limit_of_one_when_decoding_an_empty_line_stream_then_it_works() {
    let outcome = decode_with(1, &[b"\n\n\r\n"]);
    assert_eq!(outcome.frames, vec![]);
    assert_eq!(outcome.error, None);
}

#[test]
fn given_a_limit_of_one_when_decoding_a_data_line_then_it_fails() {
    let outcome = decode_with(1, &[b"data\n\n"]);
    assert_eq!(outcome.error, Some(DecodeError::FrameTooLarge { limit: 1 }));
}

#[test]
fn given_a_byte_order_mark_when_the_limit_allows_its_prefix_then_the_mark_is_not_held() {
    let outcome = decode_with(9, &[b"\xEF\xBB\xBFdata:abcd\n\n"]);
    assert_eq!(outcome.frames, vec![frame(None, "abcd")]);
    assert_eq!(outcome.error, None);
}

#[test]
fn given_a_default_decoder_when_a_line_of_one_mib_arrives_then_it_is_allowed() {
    let mut decoder = Decoder::default();
    let mut line = b"data: ".to_vec();
    line.resize(1024 * 1024, b'a');
    line.extend_from_slice(b"\n\n");
    let frames = feed_all(&mut decoder, &line).unwrap();
    assert_eq!(frames.len(), 1);
    assert_eq!(
        frames.first().map(|frame| frame.data().len()),
        Some(1024 * 1024 - 6)
    );
}

#[test]
fn given_a_default_decoder_when_the_held_bytes_pass_8_mib_then_it_fails() {
    let mut decoder = Decoder::default();
    let block = vec![b'a'; 1024 * 1024];
    for _ in 0..8 {
        assert_eq!(feed_all(&mut decoder, &block), Ok(vec![]));
    }
    assert_eq!(
        feed_all(&mut decoder, b"a"),
        Err(DecodeError::FrameTooLarge { limit: 8_388_608 })
    );
}

#[test]
fn given_an_incomplete_byte_order_mark_when_the_limit_is_one_then_the_second_byte_passes_it() {
    let outcome = decode_with(1, &[b"\xEF\xBB"]);
    assert_eq!(outcome.frames, vec![]);
    assert_eq!(outcome.error, Some(DecodeError::FrameTooLarge { limit: 1 }));
}

#[test]
fn given_a_byte_order_mark_when_the_limit_is_two_then_the_complete_mark_passes() {
    let outcome = decode_with(2, &[b"\xEF\xBB\xBFid\n"]);
    assert_eq!(outcome.frames, vec![]);
    assert_eq!(outcome.error, None);
}

#[test]
fn given_an_incomplete_byte_order_mark_at_the_limit_when_the_input_ends_then_it_is_a_held_line() {
    let outcome = decode_with(2, &[b"\xEF\xBB"]);
    assert_eq!(outcome.error, Some(DecodeError::InputTruncated));
}

#[test]
fn given_large_lines_that_completed_when_a_new_line_arrives_then_the_limit_counts_held_bytes_not_capacity()
 {
    let mut decoder = Decoder::new(1000).unwrap();
    let mut comment = b":".to_vec();
    comment.resize(900, b'x');
    comment.push(b'\n');
    for _ in 0..20 {
        assert_eq!(feed_all(&mut decoder, &comment), Ok(vec![]));
    }
    let mut data = b"data:".to_vec();
    data.resize(1000, b'y');
    assert_eq!(feed_all(&mut decoder, &data), Ok(vec![]));
    assert_eq!(feed_all(&mut decoder, b"\n\n").unwrap().len(), 1);
}

#[test]
fn given_a_latched_error_when_the_decoder_is_inspected_then_nothing_is_held() {
    let mut decoder = Decoder::new(10).unwrap();
    assert_eq!(feed_all(&mut decoder, b"event:a\n"), Ok(vec![]));
    assert!(format!("{decoder:?}").contains("held: 1,"));
    assert!(feed_all(&mut decoder, b"data: 0123456789").is_err());
    assert!(format!("{decoder:?}").contains("held: 0,"));
}
