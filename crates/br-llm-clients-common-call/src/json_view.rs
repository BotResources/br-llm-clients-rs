use std::ops::Range;

pub(crate) const DECODINGS: usize = 3;

pub(crate) struct View {
    pub(crate) bytes: Vec<u8>,
    spans: Vec<Range<usize>>,
    pub(crate) whole: usize,
    end: usize,
}

enum Step {
    Char(char, usize),
    Literal,
    Incomplete,
}

enum Hex {
    Unit(u16),
    Short,
    Bad,
}

fn hex4(bytes: &[u8]) -> Hex {
    let mut unit = 0_u16;
    for index in 0..4 {
        let Some(byte) = bytes.get(index) else {
            return Hex::Short;
        };
        let Some(digit) = char::from(*byte).to_digit(16) else {
            return Hex::Bad;
        };
        unit = unit
            .wrapping_mul(16)
            .wrapping_add(u16::try_from(digit).unwrap_or(0));
    }
    Hex::Unit(unit)
}

fn short(byte: u8) -> Option<char> {
    match byte {
        b'"' => Some('"'),
        b'\\' => Some('\\'),
        b'/' => Some('/'),
        b'b' => Some('\u{8}'),
        b'f' => Some('\u{c}'),
        b'n' => Some('\n'),
        b'r' => Some('\r'),
        b't' => Some('\t'),
        _ => None,
    }
}

fn low(high: u16, rest: &[u8]) -> Step {
    match rest {
        [] | [b'\\'] => Step::Incomplete,
        [b'\\', b'u', digits @ ..] => match hex4(digits) {
            Hex::Short => Step::Incomplete,
            Hex::Unit(low @ 0xDC00..=0xDFFF) => char::decode_utf16([high, low])
                .next()
                .and_then(Result::ok)
                .map_or(Step::Literal, |character| Step::Char(character, 12)),
            Hex::Unit(_) | Hex::Bad => Step::Literal,
        },
        _ => Step::Literal,
    }
}

fn unicode(after: &[u8]) -> Step {
    match hex4(after) {
        Hex::Short => Step::Incomplete,
        Hex::Bad => Step::Literal,
        Hex::Unit(high @ 0xD800..=0xDBFF) => low(high, after.get(4..).unwrap_or_default()),
        Hex::Unit(unit) => char::from_u32(u32::from(unit))
            .map_or(Step::Literal, |character| Step::Char(character, 6)),
    }
}

fn step(rest: &[u8]) -> Step {
    match rest {
        [b'\\'] => Step::Incomplete,
        [b'\\', b'u', after @ ..] => unicode(after),
        [b'\\', escaped, ..] => {
            short(*escaped).map_or(Step::Literal, |character| Step::Char(character, 2))
        }
        _ => Step::Literal,
    }
}

impl View {
    pub(crate) fn raw(bytes: &[u8]) -> Self {
        Self {
            bytes: bytes.to_vec(),
            spans: (0..bytes.len())
                .map(|at| at..at.saturating_add(1))
                .collect(),
            whole: bytes.len(),
            end: bytes.len(),
        }
    }

    pub(crate) fn source_at(&self, at: usize) -> usize {
        self.spans.get(at).map_or(self.end, |span| span.start)
    }

    pub(crate) fn source(&self, range: Range<usize>) -> Range<usize> {
        let start = self.source_at(range.start);
        let end = range
            .end
            .checked_sub(1)
            .and_then(|last| self.spans.get(last))
            .map_or(start, |span| span.end);
        start..end
    }

    pub(crate) fn span(&self, at: usize) -> Option<&Range<usize>> {
        self.spans.get(at)
    }

    pub(crate) fn decoded(&self, limit: usize) -> Option<Self> {
        let input = self.bytes.get(..limit)?;
        if !input.contains(&b'\\') {
            return None;
        }
        let mut bytes = Vec::with_capacity(limit);
        let mut spans = Vec::with_capacity(limit);
        let mut whole = None;
        let mut at = 0;
        while let Some(rest) = input.get(at..).filter(|rest| !rest.is_empty()) {
            let used = match step(rest) {
                Step::Char(character, used) => {
                    let span = self.source(at..at.saturating_add(used));
                    let mut buffer = [0; 4];
                    for byte in character.encode_utf8(&mut buffer).bytes() {
                        bytes.push(byte);
                        spans.push(span.clone());
                    }
                    used
                }
                other => {
                    if matches!(other, Step::Incomplete) {
                        whole.get_or_insert((bytes.len(), at));
                    }
                    bytes.extend(rest.first());
                    spans.push(self.source(at..at.saturating_add(1)));
                    1
                }
            };
            at = at.saturating_add(used);
        }
        let (whole, end) = whole.unwrap_or((bytes.len(), limit));
        Some(Self {
            end: self.source_at(end),
            bytes,
            spans,
            whole,
        })
    }
}
