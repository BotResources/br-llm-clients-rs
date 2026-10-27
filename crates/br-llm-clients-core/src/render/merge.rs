use std::collections::HashMap;

use super::refusal::invalid;
use super::types::{RenderedMessage, RenderedPart, RenderedRole};
use crate::client_error::ClientError;

pub(super) struct Item {
    role: RenderedRole,
    calls: Vec<usize>,
    results: Vec<(usize, RenderedPart)>,
    parts: Vec<RenderedPart>,
}

impl Item {
    pub(super) fn user(parts: Vec<RenderedPart>) -> Self {
        Self {
            role: RenderedRole::User,
            calls: Vec::new(),
            results: Vec::new(),
            parts,
        }
    }

    pub(super) fn assistant(parts: Vec<RenderedPart>, calls: Vec<usize>) -> Self {
        Self {
            role: RenderedRole::Assistant,
            calls,
            results: Vec::new(),
            parts,
        }
    }

    pub(super) fn results(results: Vec<(usize, RenderedPart)>) -> Self {
        Self {
            role: RenderedRole::User,
            calls: Vec::new(),
            results,
            parts: Vec::new(),
        }
    }

    fn renders_nothing(&self) -> bool {
        self.results.is_empty() && self.parts.is_empty()
    }
}

#[derive(Default)]
pub(super) struct Merge {
    messages: Vec<Open>,
    result_slots: HashMap<usize, usize>,
}

struct Open {
    role: RenderedRole,
    results: Vec<(usize, RenderedPart)>,
    parts: Vec<RenderedPart>,
}

impl Merge {
    pub(super) fn push(&mut self, item: Item) -> Result<(), ClientError> {
        if item.renders_nothing() {
            return Ok(());
        }
        for (index, part) in item.results {
            self.route(index, part)?;
        }
        if !item.parts.is_empty() {
            self.append(item.role, item.parts, &item.calls);
        }
        Ok(())
    }

    pub(super) fn finish(self) -> Vec<RenderedMessage> {
        self.messages
            .into_iter()
            .map(|mut open| {
                open.results.sort_by_key(|(index, _)| *index);
                let mut parts: Vec<RenderedPart> =
                    open.results.into_iter().map(|(_, part)| part).collect();
                parts.extend(open.parts);
                RenderedMessage::new(open.role, parts)
            })
            .collect()
    }

    fn route(&mut self, index: usize, part: RenderedPart) -> Result<(), ClientError> {
        let slot = *self
            .result_slots
            .get(&index)
            .ok_or_else(|| invalid("a tool result comes before its tool call"))?;
        if slot >= self.messages.len() {
            self.messages.push(Open {
                role: RenderedRole::User,
                results: Vec::new(),
                parts: Vec::new(),
            });
        }
        let message = self
            .messages
            .get_mut(slot)
            .ok_or_else(|| invalid("a tool result has no message to go to"))?;
        message.results.push((index, part));
        Ok(())
    }

    fn append(&mut self, role: RenderedRole, parts: Vec<RenderedPart>, calls: &[usize]) {
        match self.messages.last_mut() {
            Some(last) if last.role == role => last.parts.extend(parts),
            _ => self.messages.push(Open {
                role,
                results: Vec::new(),
                parts,
            }),
        }
        let slot = self.messages.len();
        for call in calls {
            self.result_slots.insert(*call, slot);
        }
    }
}
