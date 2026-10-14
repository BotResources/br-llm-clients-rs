use std::fmt;

use crate::decode_error::DecodeError;
use crate::frame::Frame;
use crate::line::{Line, classify};

#[derive(Default)]
pub(crate) struct OpenFrame {
    event: String,
    data: String,
    data_lines: usize,
    started: bool,
}

impl fmt::Debug for OpenFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenFrame")
            .field("held", &self.held())
            .field("data_lines", &self.data_lines)
            .field("started", &self.started)
            .finish()
    }
}

impl OpenFrame {
    pub(crate) fn held(&self) -> usize {
        self.event.len().saturating_add(self.data.len())
    }

    pub(crate) fn is_started(&self) -> bool {
        self.started
    }

    pub(crate) fn apply(&mut self, line: &[u8]) -> Result<Option<Frame>, DecodeError> {
        match classify(line) {
            Line::Empty => Ok(self.close()),
            Line::Comment => Ok(None),
            Line::Field { name, value } => {
                self.apply_field(name, value)?;
                Ok(None)
            }
        }
    }

    fn apply_field(&mut self, name: &[u8], value: &[u8]) -> Result<(), DecodeError> {
        match name {
            b"data" => {
                let text = text_of(value)?;
                if self.data_lines > 0 {
                    self.data.push('\n');
                }
                self.data.push_str(text);
                self.data_lines = self.data_lines.saturating_add(1);
                self.started = true;
            }
            b"event" => {
                self.event = text_of(value)?.to_owned();
                self.started = true;
            }
            _ => {}
        }
        Ok(())
    }

    fn close(&mut self) -> Option<Frame> {
        let frame = (self.data_lines > 0).then(|| {
            Frame::new(
                Some(std::mem::take(&mut self.event)),
                std::mem::take(&mut self.data),
            )
        });
        *self = Self::default();
        frame
    }
}

fn text_of(value: &[u8]) -> Result<&str, DecodeError> {
    std::str::from_utf8(value).map_err(|_| DecodeError::InvalidUtf8)
}
