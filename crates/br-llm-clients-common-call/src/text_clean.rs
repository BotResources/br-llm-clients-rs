use std::ops::Range;
use std::sync::OnceLock;

use br_llm_clients_common_http::Secret;

use crate::secret_automaton::Automaton;
use crate::secret_ranges::{kept_before_cut, secret_ranges};

pub(crate) const TEXT_CAP: usize = 65_536;

const PLACEHOLDER: &str = "[redacted]";

pub(crate) struct Secrets {
    secrets: Vec<Secret>,
    automaton: OnceLock<Automaton>,
}

impl Secrets {
    pub(crate) fn new(secrets: Vec<Secret>) -> Self {
        Self {
            secrets,
            automaton: OnceLock::new(),
        }
    }

    fn automaton(&self) -> &Automaton {
        self.automaton.get_or_init(|| {
            Automaton::new(self.secrets.iter().map(|secret| secret.expose().as_bytes()))
        })
    }

    pub(crate) fn ranges(&self, text: &str) -> Vec<Range<usize>> {
        secret_ranges(self.automaton(), text.as_bytes())
    }

    pub(crate) fn replace(&self, text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut copied = 0;
        for range in self.ranges(text) {
            out.push_str(text.get(copied..range.start).unwrap_or_default());
            out.push_str(PLACEHOLDER);
            copied = range.end;
        }
        out.push_str(text.get(copied..).unwrap_or_default());
        out
    }

    pub(crate) fn clean(&self, text: &str) -> String {
        cut(self.replace(text))
    }

    pub(crate) fn appears_in(&self, text: &str) -> bool {
        self.secrets.iter().any(|secret| secret.appears_in(text))
    }

    pub(crate) fn trim_cut(&self, bytes: &mut Vec<u8>) {
        let kept = kept_before_cut(self.automaton(), bytes);
        bytes.truncate(kept);
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
