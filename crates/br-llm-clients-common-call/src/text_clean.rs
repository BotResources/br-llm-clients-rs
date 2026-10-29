use std::ops::Range;
use std::sync::OnceLock;

use br_llm_clients_common_http::Secret;

use crate::secret_automaton::{Automaton, MAX_NODES};
use crate::secret_views::{kept_in_views, ranges_in_views};

pub(crate) const TEXT_CAP: usize = 65_536;

const PLACEHOLDER: &str = "[redacted]";

pub(crate) struct Secrets {
    secrets: Vec<Secret>,
    node_limit: usize,
    automaton: OnceLock<Option<Automaton>>,
}

impl Secrets {
    pub(crate) fn new(secrets: Vec<Secret>) -> Self {
        Self::with_node_limit(secrets, MAX_NODES)
    }

    pub(crate) fn with_node_limit(secrets: Vec<Secret>, node_limit: usize) -> Self {
        Self {
            secrets,
            node_limit,
            automaton: OnceLock::new(),
        }
    }

    fn automaton(&self) -> Option<&Automaton> {
        self.automaton.get_or_init(|| self.build()).as_ref()
    }

    fn build(&self) -> Option<Automaton> {
        let values: Vec<&[u8]> = self
            .secrets
            .iter()
            .map(|secret| secret.expose().as_bytes())
            .collect();
        Automaton::new(&values, self.node_limit)
    }

    #[cfg(test)]
    pub(crate) fn built(&self) -> Option<&Automaton> {
        self.automaton.get().and_then(Option::as_ref)
    }

    pub(crate) fn ranges(&self, text: &str) -> Vec<Range<usize>> {
        ranges_in_views(self.automaton(), text.as_bytes())
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
        !self.ranges(text).is_empty()
    }

    pub(crate) fn trim_cut(&self, bytes: &mut Vec<u8>) {
        if self.secrets.is_empty() {
            return;
        }
        let kept = kept_in_views(self.automaton(), bytes);
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
