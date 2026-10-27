use br_llm_clients_common_http::Secret;

pub(crate) const TEXT_CAP: usize = 65_536;

const PLACEHOLDER: &[u8] = b"[redacted]";

pub(crate) struct Secrets {
    longest_first: Vec<Secret>,
    first_bytes: Vec<bool>,
    longest: usize,
}

impl Secrets {
    pub(crate) fn new(mut secrets: Vec<Secret>) -> Self {
        secrets.sort_by_key(|secret| std::cmp::Reverse(secret.expose().len()));
        let mut first_bytes = vec![false; 256];
        for first in secrets
            .iter()
            .filter_map(|secret| secret.expose().as_bytes().first())
        {
            if let Some(known) = first_bytes.get_mut(usize::from(*first)) {
                *known = true;
            }
        }
        let longest = secrets.first().map_or(0, |secret| secret.expose().len());
        Self {
            longest_first: secrets,
            first_bytes,
            longest,
        }
    }

    pub(crate) fn replace(&self, text: &str) -> String {
        let bytes = text.as_bytes();
        let mut out = Vec::with_capacity(bytes.len());
        let mut copied = 0;
        let mut at = 0;
        while at < bytes.len() {
            match self.whole_at(bytes, at) {
                Some(length) => {
                    out.extend_from_slice(bytes.get(copied..at).unwrap_or_default());
                    out.extend_from_slice(PLACEHOLDER);
                    at = at.saturating_add(length);
                    copied = at;
                }
                None => at = at.saturating_add(1),
            }
        }
        out.extend_from_slice(bytes.get(copied..).unwrap_or_default());
        String::from_utf8_lossy(&out).into_owned()
    }

    pub(crate) fn clean(&self, text: &str) -> String {
        cut(self.replace(text))
    }

    pub(crate) fn appears_in(&self, text: &str) -> bool {
        self.longest_first
            .iter()
            .any(|secret| secret.appears_in(text))
    }

    pub(crate) fn trim_cut(&self, bytes: &mut Vec<u8>) {
        if let Some(from) = self.crossing_from(bytes) {
            bytes.truncate(from);
        }
    }

    fn crossing_from(&self, bytes: &[u8]) -> Option<usize> {
        let mut at = 0;
        while at < bytes.len() {
            if self.starts_at(bytes, at) {
                return Some(at);
            }
            at = at.saturating_add(self.whole_at(bytes, at).unwrap_or(1));
        }
        None
    }

    fn may_start(&self, byte: Option<&u8>) -> bool {
        byte.and_then(|byte| self.first_bytes.get(usize::from(*byte)))
            .copied()
            .unwrap_or(false)
    }

    fn whole_at(&self, bytes: &[u8], at: usize) -> Option<usize> {
        let rest = bytes.get(at..)?;
        if !self.may_start(rest.first()) {
            return None;
        }
        self.longest_first
            .iter()
            .map(|secret| secret.expose().as_bytes())
            .find(|secret| rest.starts_with(secret))
            .map(<[u8]>::len)
    }

    fn starts_at(&self, bytes: &[u8], at: usize) -> bool {
        let Some(rest) = bytes.get(at..) else {
            return false;
        };
        if rest.len() >= self.longest || !self.may_start(rest.first()) {
            return false;
        }
        self.longest_first.iter().any(|secret| {
            let secret = secret.expose().as_bytes();
            secret.len() > rest.len() && secret.starts_with(rest)
        })
    }
}

pub(crate) fn cut(mut text: String) -> String {
    if text.len() <= TEXT_CAP {
        return text;
    }
    let mut end = TEXT_CAP;
    while !text.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    text.truncate(end);
    text
}
