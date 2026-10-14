use crate::{DecodeError, Decoder, Frame};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Outcome {
    pub(crate) frames: Vec<Frame>,
    pub(crate) error: Option<DecodeError>,
}

pub(crate) fn decode_with(limit: usize, chunks: &[&[u8]]) -> Outcome {
    let mut decoder = Decoder::new(limit).expect("a non-zero limit");
    let mut frames = Vec::new();
    for chunk in chunks {
        if let Err(error) = decoder.feed(chunk, &mut frames) {
            return Outcome {
                frames,
                error: Some(error),
            };
        }
    }
    let error = decoder.finish().err();
    Outcome { frames, error }
}

pub(crate) fn decode(chunks: &[&[u8]]) -> Outcome {
    decode_with(crate::DEFAULT_LIMIT, chunks)
}

pub(crate) fn decode_whole(bytes: &[u8]) -> Outcome {
    decode(&[bytes])
}

pub(crate) fn frames_of(bytes: &[u8]) -> Vec<Frame> {
    let outcome = decode_whole(bytes);
    assert_eq!(outcome.error, None, "stream {bytes:?} must decode cleanly");
    outcome.frames
}

pub(crate) fn frame(event: Option<&str>, data: &str) -> Frame {
    Frame::new(event.map(str::to_owned), data)
}

pub(crate) fn feed_all(decoder: &mut Decoder, bytes: &[u8]) -> Result<Vec<Frame>, DecodeError> {
    let mut frames = Vec::new();
    decoder.feed(bytes, &mut frames)?;
    Ok(frames)
}

pub(crate) struct Xorshift(u64);

impl Xorshift {
    pub(crate) fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    pub(crate) fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    pub(crate) fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(bound).expect("bound fits")).expect("fits")
    }
}

pub(crate) fn random_chunks<'a>(bytes: &'a [u8], rng: &mut Xorshift) -> Vec<&'a [u8]> {
    let mut chunks = Vec::new();
    let mut rest = bytes;
    while !rest.is_empty() {
        let size = rng.below(6);
        let (head, tail) = rest.split_at(size.min(rest.len()));
        chunks.push(head);
        rest = tail;
    }
    chunks
}
