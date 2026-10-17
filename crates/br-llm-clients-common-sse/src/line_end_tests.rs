use crate::Decoder;
use crate::test_support::{decode, feed_all, frame, frames_of};

#[test]
fn given_line_feed_ends_when_decoding_then_the_frame_is_returned() {
    assert_eq!(frames_of(b"data: a\n\n"), vec![frame(None, "a")]);
}

#[test]
fn given_carriage_return_ends_when_decoding_then_the_frame_is_returned() {
    assert_eq!(frames_of(b"data: a\r\r"), vec![frame(None, "a")]);
}

#[test]
fn given_carriage_return_line_feed_ends_when_decoding_then_the_frame_is_returned() {
    assert_eq!(frames_of(b"data: a\r\n\r\n"), vec![frame(None, "a")]);
}

#[test]
fn given_two_carriage_returns_then_a_line_feed_when_decoding_then_the_second_return_and_the_feed_are_one_end()
 {
    assert_eq!(
        frames_of(b"data: a\r\r\ndata: b\r\r\n"),
        vec![frame(None, "a"), frame(None, "b")]
    );
}

#[test]
fn given_a_lone_line_feed_after_a_carriage_return_line_feed_when_decoding_then_it_is_an_empty_line()
{
    assert_eq!(frames_of(b"data: a\r\n\n"), vec![frame(None, "a")]);
}

#[test]
fn given_the_closing_carriage_return_when_it_arrives_then_the_frame_is_returned_at_once() {
    let mut decoder = Decoder::default();
    assert_eq!(feed_all(&mut decoder, b"data: a\r").unwrap(), vec![]);
    assert_eq!(
        feed_all(&mut decoder, b"\r").unwrap(),
        vec![frame(None, "a")]
    );
}

#[test]
fn given_a_closing_line_end_in_the_chunk_when_feeding_then_the_frame_is_in_that_chunk() {
    let mut decoder = Decoder::default();
    assert_eq!(feed_all(&mut decoder, b"data: a\n").unwrap(), vec![]);
    assert_eq!(
        feed_all(&mut decoder, b"\n").unwrap(),
        vec![frame(None, "a")]
    );
}

#[test]
fn given_a_line_feed_that_follows_a_closing_carriage_return_later_when_feeding_then_it_is_swallowed()
 {
    let mut decoder = Decoder::default();
    assert_eq!(
        feed_all(&mut decoder, b"data: a\r\r").unwrap(),
        vec![frame(None, "a")]
    );
    assert_eq!(feed_all(&mut decoder, b"\n").unwrap(), vec![]);
    assert_eq!(
        feed_all(&mut decoder, b"data: b\n\n").unwrap(),
        vec![frame(None, "b")]
    );
}

#[test]
fn given_a_chunk_of_several_frames_when_feeding_then_they_come_in_order() {
    let mut decoder = Decoder::default();
    let frames = feed_all(&mut decoder, b"data: 1\n\ndata: 2\r\rdata: 3\r\n\r\n").unwrap();
    assert_eq!(
        frames,
        vec![frame(None, "1"), frame(None, "2"), frame(None, "3")]
    );
}

#[test]
fn given_an_empty_chunk_when_feeding_then_nothing_changes() {
    let mut decoder = Decoder::default();
    assert_eq!(feed_all(&mut decoder, b"").unwrap(), vec![]);
    assert_eq!(
        feed_all(&mut decoder, b"data: a\n\n").unwrap(),
        vec![frame(None, "a")]
    );
    assert_eq!(decoder.finish(), Ok(()));
}

#[test]
fn given_an_empty_chunk_after_a_carriage_return_when_the_feed_comes_then_it_is_still_swallowed() {
    let outcome = decode(&[b"data: a\r", b"", b"\n", b"\n"]);
    assert_eq!(outcome.frames, vec![frame(None, "a")]);
    assert_eq!(outcome.error, None);
}
