use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PartKind {
    Output,
    Refusal,
}

pub(super) struct Part {
    pub(super) kind: PartKind,
    pub(super) streamed: String,
    pub(super) delta_given: bool,
}

impl Part {
    pub(super) fn new(kind: PartKind) -> Self {
        Self {
            kind,
            streamed: String::new(),
            delta_given: false,
        }
    }
}

#[derive(Default)]
pub(super) struct MessageItem {
    pub(super) parts: BTreeMap<u64, Part>,
}

#[derive(Default)]
pub(super) struct ReasoningItem {
    pub(super) open: bool,
    pub(super) text_given: bool,
    pub(super) highest_summary: Option<u64>,
}

pub(super) struct DoneCall {
    pub(super) name: Option<String>,
    pub(super) call_id: Option<String>,
    pub(super) arguments: String,
    pub(super) completed: bool,
}

pub(super) struct CallItem {
    pub(super) name: Option<String>,
    pub(super) call_id: Option<String>,
    pub(super) deltas: String,
    pub(super) done: Option<DoneCall>,
}

pub(super) enum ItemState {
    Message(MessageItem),
    Reasoning(ReasoningItem),
    Call(CallItem),
}

pub(super) struct Slot {
    pub(super) done: bool,
    pub(super) state: ItemState,
}

pub(super) type Items = BTreeMap<u64, Slot>;
