use std::time::Duration;

#[derive(Clone, Copy)]
pub enum Announce {
    Nothing,
    Unlimited,
    Limit(u32),
    Padded { filler: usize, limit: u32 },
    Raw(&'static [u8]),
    RawAfter(Duration, &'static [u8]),
    Spaced(&'static [u8], Duration, &'static [u8]),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TlsFault {
    None,
    Close,
    Hang,
}

#[derive(Clone, Copy)]
pub enum Ending {
    Eof,
    Reset,
}

#[derive(Clone, Copy)]
pub enum Answer {
    After(Duration),
    Refuse,
    RefuseAfter(Duration),
    Never,
    Frames(fn(u32) -> Vec<u8>),
}

#[derive(Clone, Copy)]
pub struct Policy {
    pub announce: Announce,
    pub answer: Answer,
    pub lower_to_after: Option<(u32, usize)>,
    pub ends_after: Option<(Duration, Ending)>,
    pub accept_delay: Duration,
    pub goaway_on_headers: Option<u32>,
    pub goaway_after: Option<(Duration, u32)>,
    pub tls_fault: TlsFault,
}

impl Policy {
    pub fn healthy() -> Self {
        Self {
            announce: Announce::Limit(100),
            answer: Answer::After(Duration::ZERO),
            lower_to_after: None,
            ends_after: None,
            accept_delay: Duration::ZERO,
            goaway_on_headers: None,
            goaway_after: None,
            tls_fault: TlsFault::None,
        }
    }

    pub fn announcing(announce: Announce) -> Self {
        Self {
            announce,
            ..Self::healthy()
        }
    }
}
