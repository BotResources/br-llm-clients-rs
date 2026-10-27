use super::types::{RenderedMessage, RenderedPart, RenderedRole};

pub(super) struct Item {
    role: RenderedRole,
    results: Vec<(usize, RenderedPart)>,
    parts: Vec<RenderedPart>,
}

impl Item {
    pub(super) fn user(parts: Vec<RenderedPart>) -> Self {
        Self {
            role: RenderedRole::User,
            results: Vec::new(),
            parts,
        }
    }

    pub(super) fn assistant(parts: Vec<RenderedPart>) -> Self {
        Self {
            role: RenderedRole::Assistant,
            results: Vec::new(),
            parts,
        }
    }

    pub(super) fn results(results: Vec<(usize, RenderedPart)>) -> Self {
        Self {
            role: RenderedRole::User,
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
    messages: Vec<RenderedMessage>,
    open: Option<Open>,
}

struct Open {
    role: RenderedRole,
    results: Vec<(usize, RenderedPart)>,
    parts: Vec<RenderedPart>,
}

impl Merge {
    pub(super) fn push(&mut self, item: Item) {
        if item.renders_nothing() {
            return;
        }
        match self.open.as_mut() {
            Some(open) if open.role == item.role => {
                open.results.extend(item.results);
                open.parts.extend(item.parts);
            }
            _ => {
                self.close();
                self.open = Some(Open {
                    role: item.role,
                    results: item.results,
                    parts: item.parts,
                });
            }
        }
    }

    pub(super) fn finish(mut self) -> Vec<RenderedMessage> {
        self.close();
        self.messages
    }

    fn close(&mut self) {
        if let Some(mut open) = self.open.take() {
            open.results.sort_by_key(|(index, _)| *index);
            let mut parts: Vec<RenderedPart> =
                open.results.into_iter().map(|(_, part)| part).collect();
            parts.extend(open.parts);
            self.messages.push(RenderedMessage::new(open.role, parts));
        }
    }
}
