use std::sync::Arc;

use crate::call_log::Served;
use crate::reply_source::ReplySource;
use crate::rule::{Matcher, Rule};

struct RuleSlot {
    matcher: Arc<Matcher>,
    reply: ReplySource,
    limit: Option<usize>,
    used: usize,
}

impl RuleSlot {
    fn has_uses_left(&self) -> bool {
        self.limit.is_none_or(|limit| self.used < limit)
    }
}

pub(crate) struct Claimed {
    pub(crate) served: Served,
    pub(crate) source: ReplySource,
    pub(crate) claimed_before: usize,
}

#[derive(Default)]
pub(crate) struct ScriptState {
    entries: Vec<ReplySource>,
    next_entry: usize,
    rules: Vec<RuleSlot>,
    fallback: Option<ReplySource>,
    fallback_claims: usize,
}

impl ScriptState {
    pub(crate) fn push_entry(&mut self, source: ReplySource) {
        self.entries.push(source);
    }

    pub(crate) fn push_rule(&mut self, rule: Rule) {
        self.rules.push(RuleSlot {
            matcher: rule.matcher,
            reply: rule.reply,
            limit: rule.limit,
            used: 0,
        });
    }

    pub(crate) fn replace_fallback(
        &mut self,
        fallback: Option<ReplySource>,
    ) -> Option<ReplySource> {
        std::mem::replace(&mut self.fallback, fallback)
    }

    pub(crate) fn rule_count(&self) -> usize {
        self.rules.len()
    }

    pub(crate) fn candidate(&self, position: usize) -> Option<Arc<Matcher>> {
        let slot = self.rules.get(position)?;
        slot.has_uses_left().then(|| Arc::clone(&slot.matcher))
    }

    pub(crate) fn claim_rule(&mut self, position: usize) -> Option<Claimed> {
        let slot = self.rules.get_mut(position)?;
        if !slot.has_uses_left() {
            return None;
        }
        let claimed_before = slot.used;
        slot.used += 1;
        Some(Claimed {
            served: Served::Rule { position },
            source: slot.reply.clone(),
            claimed_before,
        })
    }

    pub(crate) fn claim_remaining(&mut self) -> Option<Claimed> {
        if let Some(source) = self.entries.get(self.next_entry) {
            let position = self.next_entry;
            self.next_entry += 1;
            return Some(Claimed {
                served: Served::Entry { position },
                source: source.clone(),
                claimed_before: position,
            });
        }
        let source = self.fallback.clone()?;
        let claimed_before = self.fallback_claims;
        self.fallback_claims += 1;
        Some(Claimed {
            served: Served::Fallback,
            source,
            claimed_before,
        })
    }

    pub(crate) fn unused_entries(&self) -> usize {
        self.entries.len() - self.next_entry
    }

    pub(crate) fn unused_uses(&self) -> usize {
        self.rules
            .iter()
            .filter_map(|slot| slot.limit.map(|limit| limit - slot.used))
            .sum()
    }
}
