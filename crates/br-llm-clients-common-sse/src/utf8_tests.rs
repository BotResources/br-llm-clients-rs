use crate::DecodeError;
use crate::Decoder;
use crate::test_support::{decode, decode_whole, feed_all, frame};

#[test]
fn given_an_invalid_byte_in_a_comment_when_decoding_then_it_is_ignored() {
    let outcome = decode_whole(b": \xFF\xFE\xC0\xAF\ndata: ok\n\n");
    assert_eq!(outcome.frames, vec![frame(None, "ok")]);
    assert_eq!(outcome.error, None);
}

#[test]
fn given_an_invalid_byte_in_id_retry_and_unknown_fields_when_decoding_then_they_are_ignored() {
    let outcome = decode_whole(b"id: \xFF\nretry: \xFE\nfoo\xC0: \xC1\n\xFF\ndata: ok\n\n");
    assert_eq!(outcome.frames, vec![frame(None, "ok")]);
    assert_eq!(outcome.error, None);
}

#[test]
fn given_an_invalid_byte_in_a_data_line_when_the_line_completes_then_the_stream_fails() {
    let outcome = decode_whole(b"data: a\xFFb\n\n");
    assert_eq!(outcome.frames, vec![]);
    assert_eq!(outcome.error, Some(DecodeError::InvalidUtf8));
}

#[test]
fn given_an_invalid_byte_in_an_event_line_when_the_line_completes_then_the_stream_fails() {
    let outcome = decode_whole(b"event: a\xFFb\ndata: x\n\n");
    assert_eq!(outcome.frames, vec![]);
    assert_eq!(outcome.error, Some(DecodeError::InvalidUtf8));
}

#[test]
fn given_an_invalid_byte_in_an_unfinished_data_line_when_feeding_then_nothing_fails_before_the_line_end()
 {
    let mut decoder = Decoder::default();
    assert_eq!(feed_all(&mut decoder, b"data: \xFF"), Ok(vec![]));
    assert_eq!(feed_all(&mut decoder, b"more"), Ok(vec![]));
    assert_eq!(feed_all(&mut decoder, b"\n"), Err(DecodeError::InvalidUtf8));
}

#[test]
fn given_an_invalid_byte_in_a_data_line_when_the_line_ends_with_a_carriage_return_then_the_stream_fails()
 {
    let mut decoder = Decoder::default();
    assert_eq!(
        feed_all(&mut decoder, b"data: \xFF\r"),
        Err(DecodeError::InvalidUtf8)
    );
}

#[test]
fn given_a_character_cut_short_at_the_line_end_when_decoding_then_the_stream_fails() {
    let outcome = decode_whole(b"data: \xE2\x82\n\n");
    assert_eq!(outcome.error, Some(DecodeError::InvalidUtf8));
}

#[test]
fn given_overlong_and_surrogate_encodings_when_decoding_then_the_stream_fails() {
    for bad in [
        &b"\xC0\xAF"[..],
        b"\xED\xA0\x80",
        b"\xF4\x90\x80\x80",
        b"\x80",
    ] {
        let mut stream = b"data: ".to_vec();
        stream.extend_from_slice(bad);
        stream.extend_from_slice(b"\n\n");
        assert_eq!(
            decode_whole(&stream).error,
            Some(DecodeError::InvalidUtf8),
            "{bad:?}"
        );
    }
}

#[test]
fn given_a_valid_character_split_across_chunks_when_decoding_then_the_stream_is_valid() {
    let outcome = decode(&[b"data: \xE2", b"\x82", b"\xAC\n\n"]);
    assert_eq!(outcome.frames, vec![frame(None, "\u{20ac}")]);
    assert_eq!(outcome.error, None);
}

#[test]
fn given_a_character_split_across_two_data_lines_when_decoding_then_the_stream_fails() {
    let outcome = decode_whole(b"data: \xE2\x82\ndata: \xAC\n\n");
    assert_eq!(outcome.error, Some(DecodeError::InvalidUtf8));
}

#[test]
fn given_an_invalid_event_line_when_the_error_comes_then_frames_before_it_stand() {
    let outcome = decode_whole(b"data: a\n\nevent: \xFF\n");
    assert_eq!(outcome.frames, vec![frame(None, "a")]);
    assert_eq!(outcome.error, Some(DecodeError::InvalidUtf8));
}

#[test]
fn given_an_invalid_value_after_a_name_without_space_when_decoding_then_the_stream_fails() {
    assert_eq!(
        decode_whole(b"data:\xFF\n\n").error,
        Some(DecodeError::InvalidUtf8)
    );
    assert_eq!(
        decode_whole(b"event:\xFF\n\n").error,
        Some(DecodeError::InvalidUtf8)
    );
}
