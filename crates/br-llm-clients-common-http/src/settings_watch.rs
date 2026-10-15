const FRAME_HEADER: usize = 9;
const ENTRY: usize = 6;
const SETTINGS_FRAME: u8 = 4;
const ACK_FLAG: u8 = 1;
const MAX_CONCURRENT_STREAMS: u16 = 3;

#[derive(Default)]
pub(crate) struct SettingsWatch {
    head: Vec<u8>,
    remaining: usize,
    settings: bool,
    entry: Vec<u8>,
    limit: Option<u32>,
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
                if self.settings {
                    self.read_entries(taken);
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
        self.entry.clear();
        self.limit = None;
        if length == 0 {
            self.finish_frame(found);
        }
    }

    fn read_entries(&mut self, mut bytes: &[u8]) {
        if !self.entry.is_empty() {
            let need = ENTRY.saturating_sub(self.entry.len());
            let (taken, rest) = bytes.split_at(need.min(bytes.len()));
            self.entry.extend_from_slice(taken);
            bytes = rest;
            if self.entry.len() < ENTRY {
                return;
            }
            let carried = std::mem::take(&mut self.entry);
            if let Some(whole) = carried.first_chunk::<ENTRY>() {
                self.take_entry(*whole);
            }
        }
        let (whole, tail) = bytes.as_chunks::<ENTRY>();
        for entry in whole {
            self.take_entry(*entry);
        }
        self.entry.extend_from_slice(tail);
    }

    fn take_entry(&mut self, [id_high, id_low, v3, v2, v1, v0]: [u8; ENTRY]) {
        if u16::from_be_bytes([id_high, id_low]) == MAX_CONCURRENT_STREAMS {
            self.limit = Some(u32::from_be_bytes([v3, v2, v1, v0]));
        }
    }

    fn finish_frame(&mut self, found: &mut impl FnMut(Option<u32>)) {
        if self.settings {
            found(self.limit.take());
        }
        self.settings = false;
        self.entry.clear();
        self.limit = None;
        self.head.clear();
    }
}

#[cfg(test)]
#[path = "settings_watch_tests.rs"]
mod tests;
