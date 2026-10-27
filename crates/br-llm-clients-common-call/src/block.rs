use std::collections::VecDeque;

use br_llm_clients_core::IdSource;
use br_llm_messages::{
    BlockId, BlockStart, DeltaOp, EntryId, ExternalId, Finalization, OpaqueJson, ReplayFormat,
    ResponseDelta, ToolName, TurnId,
};

use crate::judge::{Held, Json};

#[derive(Clone)]
pub(crate) enum Kind {
    Text,
    Thinking,
    Redacted,
    Structured,
    ToolCall {
        name: ToolName,
        external_id: Option<ExternalId>,
    },
}

#[derive(Clone)]
pub(crate) enum Op {
    Start,
    Text(String),
    Thinking(String),
    Replay { format: ReplayFormat, text: String },
    Json(String),
    End,
}

pub(crate) struct Emit<'a> {
    pub(crate) ids: &'a dyn IdSource,
    pub(crate) turn: TurnId,
    pub(crate) entry: EntryId,
    pub(crate) out: &'a mut Vec<ResponseDelta>,
}

impl Emit<'_> {
    fn push(&mut self, op: DeltaOp) {
        self.out.push(ResponseDelta::new(self.turn, self.entry, op));
    }
}

#[derive(Clone)]
pub(crate) struct Block {
    pub(crate) key: String,
    kind: Kind,
    pub(crate) closed: bool,
    provider_closed: bool,
    finished: bool,
    queue: VecDeque<Op>,
    id: Option<BlockId>,
    held: Vec<Op>,
    json: String,
    nonblank: bool,
}

pub(crate) fn is_blank(text: &str) -> bool {
    text.chars().all(char::is_whitespace)
}

impl Block {
    pub(crate) fn new(key: String, kind: Kind) -> Self {
        Self {
            key,
            kind,
            closed: false,
            provider_closed: false,
            finished: false,
            queue: VecDeque::from([Op::Start]),
            id: None,
            held: Vec::new(),
            json: String::new(),
            nonblank: false,
        }
    }

    pub(crate) fn accepts(&self, op: &Op) -> bool {
        matches!(
            (&self.kind, op),
            (Kind::Text, Op::Text(_))
                | (Kind::Thinking, Op::Thinking(_) | Op::Replay { .. })
                | (Kind::Redacted, Op::Replay { .. })
                | (Kind::Structured, Op::Json(_))
                | (Kind::ToolCall { .. }, Op::Json(_))
        )
    }

    pub(crate) fn push(&mut self, op: Op) {
        match &op {
            Op::Text(text) => self.nonblank = self.nonblank || !is_blank(text),
            Op::Json(json) => self.json.push_str(json),
            Op::End => {
                self.closed = true;
                self.provider_closed = true;
            }
            _ => {}
        }
        self.queue.push_back(op);
    }

    pub(crate) fn push_forced_end(&mut self) {
        self.closed = true;
        self.queue.push_back(Op::End);
    }

    pub(crate) fn is_finished(&self) -> bool {
        self.finished
    }

    pub(crate) fn drain(&mut self, cx: &mut Emit<'_>) {
        while let Some(op) = self.queue.pop_front() {
            match op {
                Op::Start => self.start(cx),
                Op::End => self.close(cx),
                other => self.deliver(other, cx),
            }
        }
    }

    pub(crate) fn held(&self) -> Held {
        match &self.kind {
            Kind::Text => Held::Text {
                nonblank: self.nonblank,
            },
            Kind::Thinking => Held::Thinking,
            Kind::Redacted => Held::Redacted,
            Kind::Structured => Held::Structured(self.json_state()),
            Kind::ToolCall { .. } => Held::ToolCall(self.json_state()),
        }
    }

    fn json_state(&self) -> Json {
        if !self.provider_closed {
            return Json::Incomplete;
        }
        match serde_json::from_str::<serde_json::Value>(&self.json) {
            Err(error) if error.is_eof() => Json::Incomplete,
            Err(error) => Json::Malformed(error.to_string()),
            Ok(value) => self.strict(&value),
        }
    }

    fn strict(&self, value: &serde_json::Value) -> Json {
        if matches!(self.kind, Kind::ToolCall { .. }) && !value.is_object() {
            return Json::Malformed("the arguments of a tool call are not an object".to_owned());
        }
        match OpaqueJson::parse(&self.json) {
            Ok(_) => Json::Complete,
            Err(error) => Json::Malformed(error.to_string()),
        }
    }

    fn start(&mut self, cx: &mut Emit<'_>) {
        if !matches!(self.kind, Kind::Text | Kind::Thinking) {
            self.open(cx);
        }
    }

    fn deliver(&mut self, op: Op, cx: &mut Emit<'_>) {
        if self.id.is_none() {
            if !self.qualifies(&op) {
                self.held.push(op);
                return;
            }
            self.open(cx);
            for held in std::mem::take(&mut self.held) {
                self.send(held, cx);
            }
        }
        self.send(op, cx);
    }

    fn qualifies(&self, op: &Op) -> bool {
        match (&self.kind, op) {
            (Kind::Text, Op::Text(text)) => !is_blank(text),
            (Kind::Thinking, Op::Thinking(text)) => !text.is_empty(),
            (Kind::Thinking, Op::Replay { text, .. }) => !text.is_empty(),
            _ => true,
        }
    }

    fn close(&mut self, cx: &mut Emit<'_>) {
        self.finished = true;
        let Some(block) = self.id else {
            return;
        };
        let finalize = match self.kind {
            Kind::Text | Kind::Thinking | Kind::Redacted => true,
            Kind::Structured | Kind::ToolCall { .. } => self.json_state() == Json::Complete,
        };
        if finalize {
            cx.push(DeltaOp::block_finalized(block, Finalization::accumulated()));
        }
    }

    fn open(&mut self, cx: &mut Emit<'_>) {
        let block = cx.ids.next_block_id();
        self.id = Some(block);
        let start = match &self.kind {
            Kind::Text => BlockStart::text(),
            Kind::Thinking => BlockStart::thinking(),
            Kind::Redacted => BlockStart::redacted_thinking(),
            Kind::Structured => BlockStart::structured_content(),
            Kind::ToolCall { name, external_id } => {
                BlockStart::runtime_tool_call(Some(name.clone()), external_id.clone())
            }
        };
        cx.push(DeltaOp::block_started(block, start));
    }

    fn send(&self, op: Op, cx: &mut Emit<'_>) {
        let Some(block) = self.id else {
            return;
        };
        match op {
            Op::Text(text) => cx.push(DeltaOp::text_appended(block, text)),
            Op::Thinking(text) => cx.push(DeltaOp::thinking_appended(block, text)),
            Op::Replay { format, text } => {
                cx.push(DeltaOp::replay_fragment_appended(block, format, text));
            }
            Op::Json(fragment) => cx.push(DeltaOp::json_appended(block, fragment)),
            Op::Start | Op::End => {}
        }
    }
}
