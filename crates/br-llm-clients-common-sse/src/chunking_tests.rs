use crate::test_support::{Xorshift, decode, decode_whole, frame, random_chunks};

fn corpus() -> Vec<(&'static str, Vec<u8>)> {
    let invalid_data: &[u8] = b"data: a\n\ndata: \xFF\n\ndata: late\n\n";
    let invalid_comment: &[u8] = b": \xFF\xFE\ndata: ok\n\n";
    vec![
        ("plain", b"data: one\n\ndata: two\n\n".to_vec()),
        ("multi-line data", b"data: a\ndata: b\ndata: c\n\n".to_vec()),
        ("named events", b"event: add\ndata: 1\n\nevent: remove\ndata: 2\n\n".to_vec()),
        ("crlf", b"event: x\r\ndata: y\r\n\r\ndata: z\r\n\r\n".to_vec()),
        ("cr only", b"event: x\rdata: y\r\rdata: z\r\r".to_vec()),
        ("cr cr lf", b"data: a\r\r\ndata: b\r\n\r\n".to_vec()),
        ("mixed ends", b"data: a\ndata: b\r\ndata: c\r\n\r".to_vec()),
        ("bom", b"\xEF\xBB\xBFdata: after bom\n\n".to_vec()),
        ("bom then crlf", b"\xEF\xBB\xBF\r\ndata: x\r\n\r\n".to_vec()),
        ("bom lookalike", b"\xEF\xBBdata: x\n\ndata: y\n\n".to_vec()),
        ("bom twice", b"\xEF\xBB\xBF\xEF\xBB\xBFdata: x\n\n".to_vec()),
        ("utf8", "data: caf\u{e9} \u{20ac} \u{1f600}\n\nevent: \u{e9}t\u{e9}\ndata: \u{4e2d}\u{6587}\n\n".as_bytes().to_vec()),
        ("comments", b": keep-alive\n: another\ndata: x\n\n: tail\n\n".to_vec()),
        ("ignored fields", b"id: 7\nretry: 100\nunknown: z\nData: no\ndata: kept\n\n".to_vec()),
        ("no colon", b"data\ndata\n\nevent\ndata: x\n\n".to_vec()),
        ("space rule", b"data:  two spaces\ndata:none\n\n".to_vec()),
        ("event reset", b"event: a\n\ndata: x\n\n".to_vec()),
        ("truncated frame", b"data: unfinished\n".to_vec()),
        ("truncated line", b"data: x\n\ndata: unfin".to_vec()),
        ("truncated bom", b"\xEF\xBB".to_vec()),
        ("invalid data", invalid_data.to_vec()),
        ("invalid comment", invalid_comment.to_vec()),
        ("invalid event", b"event: \xC3\ndata: x\n\n".to_vec()),
    ]
}

#[test]
fn given_a_stream_when_it_is_split_at_every_byte_boundary_then_the_outcome_is_unchanged() {
    for (name, bytes) in corpus() {
        let expected = decode_whole(&bytes);
        for cut in 0..=bytes.len() {
            let (head, tail) = bytes.split_at(cut);
            assert_eq!(decode(&[head, tail]), expected, "{name} cut at {cut}");
        }
    }
}

#[test]
fn given_a_stream_when_it_is_split_at_every_pair_of_boundaries_then_the_outcome_is_unchanged() {
    for (name, bytes) in corpus() {
        let expected = decode_whole(&bytes);
        for first in 0..=bytes.len() {
            for second in first..=bytes.len() {
                let (head, rest) = bytes.split_at(first);
                let (middle, tail) = rest.split_at(second - first);
                assert_eq!(
                    decode(&[head, middle, tail]),
                    expected,
                    "{name} cut at {first} and {second}"
                );
            }
        }
    }
}

#[test]
fn given_a_stream_when_it_is_fed_one_byte_at_a_time_then_the_outcome_is_unchanged() {
    for (name, bytes) in corpus() {
        let expected = decode_whole(&bytes);
        let singles: Vec<&[u8]> = bytes.chunks(1).collect();
        assert_eq!(decode(&singles), expected, "{name}");
    }
}

#[test]
fn given_a_stream_when_it_is_cut_in_random_chunks_then_the_outcome_is_unchanged() {
    let mut rng = Xorshift::new(0x9E37_79B9_7F4A_7C15);
    for (name, bytes) in corpus() {
        let expected = decode_whole(&bytes);
        for round in 0..300 {
            let chunks = random_chunks(&bytes, &mut rng);
            assert_eq!(
                decode(&chunks),
                expected,
                "{name} round {round}: {chunks:?}"
            );
        }
    }
}

#[test]
fn given_empty_chunks_between_the_bytes_when_decoding_then_the_outcome_is_unchanged() {
    for (name, bytes) in corpus() {
        let expected = decode_whole(&bytes);
        let mut chunks: Vec<&[u8]> = vec![b""];
        for single in bytes.chunks(1) {
            chunks.push(single);
            chunks.push(b"");
        }
        assert_eq!(decode(&chunks), expected, "{name}");
    }
}

#[test]
fn given_a_multibyte_character_split_inside_when_decoding_then_the_text_is_whole() {
    let bytes = "data: \u{1f600}\n\n".as_bytes();
    let expected = vec![frame(None, "\u{1f600}")];
    for cut in 7..11 {
        let (head, tail) = bytes.split_at(cut);
        let outcome = decode(&[head, tail]);
        assert_eq!(outcome.frames, expected, "cut at {cut}");
        assert_eq!(outcome.error, None);
    }
}

#[test]
fn given_a_carriage_return_and_its_line_feed_in_two_chunks_when_decoding_then_there_is_one_line_end()
 {
    let outcome = decode(&[b"data: a\r", b"\n", b"data: b\r", b"\n\r", b"\n"]);
    assert_eq!(outcome.frames, vec![frame(None, "a\nb")]);
    assert_eq!(outcome.error, None);
}

#[test]
fn given_a_field_name_split_inside_when_decoding_then_the_field_is_recognised() {
    let outcome = decode(&[b"ev", b"ent: n\nda", b"ta: v\n", b"\n"]);
    assert_eq!(outcome.frames, vec![frame(Some("n"), "v")]);
    assert_eq!(outcome.error, None);
}

#[test]
fn given_a_byte_order_mark_split_inside_when_decoding_then_it_is_still_skipped() {
    for cut in 1..3 {
        let bom: &[u8] = b"\xEF\xBB\xBF";
        let (head, tail) = bom.split_at(cut);
        let outcome = decode(&[head, tail, b"data: x\n\n"]);
        assert_eq!(outcome.frames, vec![frame(None, "x")], "cut at {cut}");
        assert_eq!(outcome.error, None);
    }
}

#[test]
fn given_a_byte_order_mark_split_from_what_follows_when_decoding_then_the_field_name_is_clean() {
    let outcome = decode(&[b"\xEF\xBB\xBF", b"data: x\n\n"]);
    assert_eq!(outcome.frames, vec![frame(None, "x")]);
}
