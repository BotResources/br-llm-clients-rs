const FRAME_HEADER: usize = 9;
const SETTINGS_FRAME: u8 = 4;
const ACK_FLAG: u8 = 1;
const MAX_CONCURRENT_STREAMS: u16 = 3;
const MAX_CAPTURED_PAYLOAD: usize = 1024;

#[derive(Default)]
pub(crate) struct SettingsWatch {
    head: Vec<u8>,
    remaining: usize,
    settings: bool,
    payload: Option<Vec<u8>>,
}

impl SettingsWatch {
    pub(crate) fn feed(&mut self, mut bytes: &[u8], found: &mut impl FnMut(Option<u32>)) {
        while !bytes.is_empty() {
            if self.remaining == 0 {
                let need = FRAME_HEADER.saturating_sub(self.head.len());
                let (taken, rest) = bytes.split_at(need.min(bytes.len()));
                self.head.extend_from_slice(taken);
                bytes = rest;
                if self.head.len() == FRAME_HEADER {
                    self.begin_frame(found);
                }
            } else {
                let (taken, rest) = bytes.split_at(self.remaining.min(bytes.len()));
                if let Some(payload) = &mut self.payload {
                    payload.extend_from_slice(taken);
                }
                self.remaining = self.remaining.saturating_sub(taken.len());
                bytes = rest;
                if self.remaining == 0 {
                    self.finish_frame(found);
                }
            }
        }
    }

    fn begin_frame(&mut self, found: &mut impl FnMut(Option<u32>)) {
        let length = self.head.iter().take(3).fold(0usize, |acc, byte| {
            acc.saturating_mul(256).saturating_add(usize::from(*byte))
        });
        let kind = self.head.get(3).copied();
        let flags = self.head.get(4).copied().unwrap_or_default();
        self.settings = kind == Some(SETTINGS_FRAME) && flags & ACK_FLAG == 0;
        self.remaining = length;
        self.payload = (self.settings && length <= MAX_CAPTURED_PAYLOAD).then(Vec::new);
        if length == 0 {
            self.finish_frame(found);
        }
    }

    fn finish_frame(&mut self, found: &mut impl FnMut(Option<u32>)) {
        if self.settings {
            let payload = self.payload.take();
            found(payload.and_then(|entries| max_concurrent_streams(&entries)));
        }
        self.settings = false;
        self.payload = None;
        self.head.clear();
    }
}

fn max_concurrent_streams(entries: &[u8]) -> Option<u32> {
    entries
        .as_chunks::<6>()
        .0
        .iter()
        .filter_map(|&[id_high, id_low, v3, v2, v1, v0]| {
            let id = u16::from_be_bytes([id_high, id_low]);
            (id == MAX_CONCURRENT_STREAMS).then(|| u32::from_be_bytes([v3, v2, v1, v0]))
        })
        .next_back()
}

#[cfg(test)]
mod tests {
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
    fn given_a_settings_frame_larger_than_the_capture_limit_then_it_is_seen_without_a_limit() {
        let big = vec![0u8; MAX_CAPTURED_PAYLOAD + 6];
        let mut bytes = frame(4, 0, &big);
        bytes.extend_from_slice(&frame(4, 0, &limit(3)));
        assert_eq!(watched(&[&bytes]), [None, Some(3)]);
    }
}
