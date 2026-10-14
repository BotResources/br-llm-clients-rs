use std::fmt;

use crate::decode_error::DecodeError;
use crate::frame::Frame;
use crate::open_frame::OpenFrame;

pub const DEFAULT_LIMIT: usize = 8_388_608;

const BYTE_ORDER_MARK: [u8; 3] = [0xEF, 0xBB, 0xBF];
const CR: u8 = b'\r';
const LF: u8 = b'\n';
const RETAINED_LINE_CAPACITY: usize = 4096;

impl fmt::Debug for Decoder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Decoder")
            .field("limit", &self.limit)
            .field("held", &self.held())
            .field("at_start", &self.at_start)
            .field("skip_lf", &self.skip_lf)
            .field("ended", &self.ended)
            .field("failed", &self.failed)
            .finish()
    }
}

pub struct Decoder {
    limit: usize,
    line: Vec<u8>,
    frame: OpenFrame,
    at_start: bool,
    skip_lf: bool,
    ended: bool,
    failed: Option<DecodeError>,
}

impl Default for Decoder {
    fn default() -> Self {
        Self::with_limit(DEFAULT_LIMIT)
    }
}

impl Decoder {
    pub fn new(limit: usize) -> Result<Self, DecodeError> {
        if limit == 0 {
            return Err(DecodeError::InvalidLimit);
        }
        Ok(Self::with_limit(limit))
    }

    fn with_limit(limit: usize) -> Self {
        Self {
            limit,
            line: Vec::new(),
            frame: OpenFrame::default(),
            at_start: true,
            skip_lf: false,
            ended: false,
            failed: None,
        }
    }

    pub fn feed(&mut self, chunk: &[u8], frames: &mut Vec<Frame>) -> Result<(), DecodeError> {
        if let Some(error) = &self.failed {
            return Err(error.clone());
        }
        if self.ended {
            return Err(DecodeError::InputEnded);
        }
        let result = self.consume(chunk, frames);
        if let Err(error) = &result {
            self.latch(error.clone());
        }
        result
    }

    pub fn finish(&mut self) -> Result<(), DecodeError> {
        if let Some(error) = &self.failed {
            return Err(error.clone());
        }
        if self.frame.is_started() || !self.line.is_empty() {
            self.latch(DecodeError::InputTruncated);
            return Err(DecodeError::InputTruncated);
        }
        self.ended = true;
        Ok(())
    }

    fn consume(&mut self, chunk: &[u8], frames: &mut Vec<Frame>) -> Result<(), DecodeError> {
        let mut rest = chunk;
        while let Some((&first, tail)) = rest.split_first() {
            if self.skip_lf {
                self.skip_lf = false;
                if first == LF {
                    rest = tail;
                    continue;
                }
            }
            if self.at_start && self.consume_start_byte(first)? {
                rest = tail;
                continue;
            }
            let (segment, after) = match rest.iter().position(|byte| is_line_end(*byte)) {
                Some(end) => rest.split_at(end),
                None => (rest, &[][..]),
            };
            self.hold(segment)?;
            rest = match after.split_first() {
                Some((&terminator, tail)) => {
                    self.complete_line(frames)?;
                    self.skip_lf = terminator == CR;
                    tail
                }
                None => &[],
            };
        }
        Ok(())
    }

    fn consume_start_byte(&mut self, byte: u8) -> Result<bool, DecodeError> {
        let expected = BYTE_ORDER_MARK.get(self.line.len()).copied();
        if expected != Some(byte) {
            self.at_start = false;
            return Ok(false);
        }
        if self.line.len().saturating_add(1) == BYTE_ORDER_MARK.len() {
            self.line.clear();
            self.at_start = false;
        } else {
            self.hold(&[byte])?;
        }
        Ok(true)
    }

    fn latch(&mut self, error: DecodeError) {
        self.failed = Some(error);
        self.line = Vec::new();
        self.frame = OpenFrame::default();
    }

    fn held(&self) -> usize {
        self.line.len().saturating_add(self.frame.held())
    }

    fn hold(&mut self, bytes: &[u8]) -> Result<(), DecodeError> {
        if self.held().saturating_add(bytes.len()) > self.limit {
            return Err(DecodeError::FrameTooLarge { limit: self.limit });
        }
        self.line.extend_from_slice(bytes);
        Ok(())
    }

    fn complete_line(&mut self, frames: &mut Vec<Frame>) -> Result<(), DecodeError> {
        let outcome = self.frame.apply(&self.line);
        self.line.clear();
        self.line.shrink_to(RETAINED_LINE_CAPACITY);
        frames.extend(outcome?);
        Ok(())
    }
}

fn is_line_end(byte: u8) -> bool {
    byte == CR || byte == LF
}
