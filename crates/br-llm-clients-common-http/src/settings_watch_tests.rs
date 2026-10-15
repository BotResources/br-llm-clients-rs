use super::*;

fn frame(kind: u8, flags: u8, payload: &[u8]) -> Vec<u8> {
    let length = payload.len();
    let mut out = vec![
        (length >> 16) as u8,
        (length >> 8) as u8,
        length as u8,
        kind,
        flags,
    ];
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(payload);
    out
}

fn limit(value: u32) -> Vec<u8> {
    let mut entry = MAX_CONCURRENT_STREAMS.to_be_bytes().to_vec();
    entry.extend_from_slice(&value.to_be_bytes());
    entry
}

fn filler(entries: usize) -> Vec<u8> {
    (0..entries)
        .flat_map(|_| [0x77, 0x77, 0, 0, 0, 9])
        .collect()
}

fn watched(chunks: &[&[u8]]) -> Vec<Option<u32>> {
    let mut watch = SettingsWatch::default();
    let mut seen = Vec::new();
    for chunk in chunks {
        watch.feed(chunk, &mut |max| seen.push(max));
    }
    seen
}

#[test]
fn given_a_settings_frame_with_a_limit_then_the_limit_is_found() {
    assert_eq!(watched(&[&frame(4, 0, &limit(100))]), [Some(100)]);
}

#[test]
fn given_a_limit_of_zero_then_zero_is_found_not_nothing() {
    assert_eq!(watched(&[&frame(4, 0, &limit(0))]), [Some(0)]);
}

#[test]
fn given_an_empty_settings_frame_then_it_is_seen_without_a_limit() {
    assert_eq!(watched(&[&frame(4, 0, &[])]), [None]);
}

#[test]
fn given_other_entries_before_the_limit_then_the_limit_is_still_found() {
    let mut entries = vec![0, 4, 0, 0, 255, 255];
    entries.extend_from_slice(&limit(7));
    assert_eq!(watched(&[&frame(4, 0, &entries)]), [Some(7)]);
}

#[test]
fn given_a_settings_ack_or_another_frame_then_nothing_is_found() {
    let mut bytes = frame(4, 1, &[]);
    bytes.extend_from_slice(&frame(6, 0, &[0; 8]));
    bytes.extend_from_slice(&frame(0, 0, &limit(5)));
    assert!(watched(&[&bytes]).is_empty());
}

#[test]
fn given_frames_split_at_every_byte_then_the_same_limits_are_found_in_order() {
    let mut bytes = frame(4, 0, &limit(100));
    bytes.extend_from_slice(&frame(0, 0, b"data"));
    bytes.extend_from_slice(&frame(4, 0, &limit(0)));
    let chunks: Vec<&[u8]> = bytes.chunks(1).collect();
    assert_eq!(watched(&chunks), [Some(100), Some(0)]);
}

#[test]
fn given_a_settings_frame_of_any_legal_size_ending_with_a_limit_then_the_limit_is_found() {
    for entries in [0usize, 1, 170, 172, 2730, 2729] {
        let mut payload = filler(entries);
        payload.extend_from_slice(&limit(1));
        let bytes = frame(4, 0, &payload);
        assert_eq!(watched(&[&bytes]), [Some(1)], "{entries} filler entries");
        let one_by_one: Vec<&[u8]> = bytes.chunks(1).collect();
        assert_eq!(watched(&one_by_one), [Some(1)], "{entries} filler entries");
        let sevens: Vec<&[u8]> = bytes.chunks(7).collect();
        assert_eq!(watched(&sevens), [Some(1)], "{entries} filler entries");
    }
}

#[test]
fn given_a_settings_frame_without_a_limit_among_many_entries_then_it_is_seen_without_a_limit() {
    let bytes = frame(4, 0, &filler(2000));
    assert_eq!(watched(&[&bytes]), [None]);
}

#[test]
fn given_a_limit_repeated_in_one_frame_then_the_last_one_wins() {
    let mut payload = limit(5);
    payload.extend_from_slice(&limit(0));
    assert_eq!(watched(&[&frame(4, 0, &payload)]), [Some(0)]);
}

#[test]
fn given_a_payload_that_is_not_a_multiple_of_six_then_only_whole_entries_count() {
    let mut payload = limit(9);
    payload.extend_from_slice(&[0, 3, 0, 0]);
    assert_eq!(watched(&[&frame(4, 0, &payload)]), [Some(9)]);
}

#[test]
fn given_a_limit_in_a_frame_that_follows_a_huge_settings_frame_then_it_is_found() {
    let mut bytes = frame(4, 0, &filler(2730));
    bytes.extend_from_slice(&frame(4, 0, &limit(3)));
    assert_eq!(watched(&[&bytes]), [None, Some(3)]);
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(bound).unwrap_or(1)).unwrap_or(0)
    }
}

fn random_entries(rng: &mut Rng) -> Vec<u8> {
    let mut payload = Vec::new();
    let bound = if rng.below(8) == 0 { 2730 } else { 12 };
    for _ in 0..rng.below(bound) {
        let id = match rng.below(4) {
            0 => MAX_CONCURRENT_STREAMS,
            1 => 4,
            2 => 0x7777,
            _ => u16::try_from(rng.below(10)).unwrap_or(0),
        };
        payload.extend_from_slice(&id.to_be_bytes());
        payload.extend_from_slice(&u32::try_from(rng.next() >> 40).unwrap_or(0).to_be_bytes());
    }
    for _ in 0..rng.below(3) * rng.below(2) {
        payload.push(u8::try_from(rng.below(256)).unwrap_or(0));
    }
    payload
}

fn random_stream(rng: &mut Rng) -> Vec<u8> {
    let mut bytes = Vec::new();
    for _ in 0..1 + rng.below(6) {
        let (kind, flags, payload) = match rng.below(5) {
            0 | 1 => (4, 0, random_entries(rng)),
            2 => (4, 1, Vec::new()),
            3 => (4, 1, random_entries(rng)),
            _ => (
                u8::try_from(rng.below(10)).unwrap_or(0),
                0,
                (0..rng.below(40))
                    .map(|_| u8::try_from(rng.below(256)).unwrap_or(0))
                    .collect(),
            ),
        };
        bytes.extend_from_slice(&frame(kind, flags, &payload));
    }
    let cut = bytes.len().saturating_sub(rng.below(3) * rng.below(30));
    bytes.truncate(cut);
    bytes
}

fn reference(bytes: &[u8]) -> Vec<Option<u32>> {
    let mut seen = Vec::new();
    let mut at = 0usize;
    while let Some(head) = bytes.get(at..at + FRAME_HEADER) {
        let length =
            (usize::from(head[0]) << 16) | (usize::from(head[1]) << 8) | usize::from(head[2]);
        let Some(payload) = bytes.get(at + FRAME_HEADER..at + FRAME_HEADER + length) else {
            break;
        };
        if head[3] == SETTINGS_FRAME && head[4] & ACK_FLAG == 0 {
            seen.push(
                payload
                    .as_chunks::<ENTRY>()
                    .0
                    .iter()
                    .filter(|entry| entry[0] == 0 && entry[1] == 3)
                    .map(|entry| u32::from_be_bytes([entry[2], entry[3], entry[4], entry[5]]))
                    .next_back(),
            );
        }
        at += FRAME_HEADER + length;
    }
    seen
}

#[test]
fn given_random_frame_streams_split_at_random_points_then_every_limit_is_read_exactly() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for round in 0..3000 {
        let bytes = random_stream(&mut rng);
        let expected = reference(&bytes);
        let mut chunks: Vec<&[u8]> = Vec::new();
        let mut rest = bytes.as_slice();
        while !rest.is_empty() {
            let size = match rng.below(4) {
                0 => 1,
                1 => 1 + rng.below(9),
                2 => 1 + rng.below(200),
                _ => rest.len(),
            };
            let (taken, tail) = rest.split_at(size.min(rest.len()));
            chunks.push(taken);
            rest = tail;
        }
        assert_eq!(watched(&chunks), expected, "round {round}");
    }
}
