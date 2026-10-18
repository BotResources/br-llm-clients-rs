use std::collections::VecDeque;

use br_llm_messages::{BlockKind, StreamEvent, Text, ToolCallId, ToolName};

use crate::judge::{Held, Json};

pub(crate) enum Kind {
    Text,
    Thinking,
    Redacted,
    Structured,
    ToolCall { id: ToolCallId, name: ToolName },
}

pub(crate) enum Raw {
    Start,
    Text(String),
    Thinking(String),
    Signature(String),
    Structured(String),
    Arguments(String),
    Redacted(String),
    End,
}

pub(crate) struct Block {
    pub(crate) key: String,
    kind: Kind,
    pub(crate) end_queued: bool,
    provider_closed: bool,
    finished: bool,
    queue: VecDeque<Raw>,
    index: Option<usize>,
    held_blank: Vec<String>,
    json: String,
    has_text: bool,
}

pub(crate) fn is_blank(text: &str) -> bool {
    Text::new(text).is_err()
}

impl Block {
    pub(crate) fn new(key: String, kind: Kind) -> Self {
        let mut queue = VecDeque::new();
        queue.push_back(Raw::Start);
        Self {
            key,
            kind,
            end_queued: false,
            provider_closed: false,
            finished: false,
            queue,
            index: None,
            held_blank: Vec::new(),
            json: String::new(),
            has_text: false,
        }
    }

    pub(crate) fn accepts(&self, raw: &Raw) -> bool {
        matches!(
            (&self.kind, raw),
            (Kind::Text, Raw::Text(_))
                | (Kind::Thinking, Raw::Thinking(_) | Raw::Signature(_))
                | (Kind::Redacted, Raw::Redacted(_))
                | (Kind::Structured, Raw::Structured(_))
                | (Kind::ToolCall { .. }, Raw::Arguments(_))
        )
    }

    pub(crate) fn push(&mut self, raw: Raw) {
        match &raw {
            Raw::Text(text) => self.has_text = self.has_text || !is_blank(text),
            Raw::Structured(json) | Raw::Arguments(json) => self.json.push_str(json),
            Raw::End => {
                self.end_queued = true;
                self.provider_closed = true;
            }
            _ => {}
        }
        self.queue.push_back(raw);
    }

    pub(crate) fn push_forced_end(&mut self) {
        self.end_queued = true;
        self.queue.push_back(Raw::End);
    }

    pub(crate) fn is_finished(&self) -> bool {
        self.finished
    }

    pub(crate) fn drain(&mut self, next_index: &mut usize, out: &mut Vec<StreamEvent>) {
        while let Some(raw) = self.queue.pop_front() {
            self.flow(raw, next_index, out);
        }
    }

    pub(crate) fn held(&self) -> Held {
        match &self.kind {
            Kind::Text => Held::Text {
                nonblank: self.has_text,
            },
            Kind::Thinking => Held::Thinking,
            Kind::Redacted => Held::Redacted,
            Kind::Structured => Held::Structured(self.json_state()),
            Kind::ToolCall { .. } => Held::ToolCall(self.json_state()),
        }
    }

    fn json_state(&self) -> Json {
        if !self.provider_closed || self.json.is_empty() {
            return Json::Incomplete;
        }
        match serde_json::from_str::<serde_json::Value>(&self.json) {
            Ok(_) => Json::Complete,
            Err(error) if error.is_eof() => Json::Incomplete,
            Err(error) => Json::Malformed(error.to_string()),
        }
    }

    fn flow(&mut self, raw: Raw, next_index: &mut usize, out: &mut Vec<StreamEvent>) {
        match raw {
            Raw::Start => {
                if !matches!(self.kind, Kind::Text) {
                    self.open(next_index, out);
                }
            }
            Raw::Text(text) => self.flow_text(text, next_index, out),
            Raw::End => {
                if let Some(index) = self.index {
                    out.push(StreamEvent::BlockEnd { index });
                }
                self.finished = true;
            }
            delta => {
                if let Some(index) = self.index {
                    out.extend(delta_event(index, delta));
                }
            }
        }
    }

    fn flow_text(&mut self, text: String, next_index: &mut usize, out: &mut Vec<StreamEvent>) {
        if self.index.is_none() {
            if is_blank(&text) {
                self.held_blank.push(text);
                return;
            }
            self.open(next_index, out);
        }
        if let Some(index) = self.index {
            for held in std::mem::take(&mut self.held_blank) {
                out.push(StreamEvent::TextDelta { index, text: held });
            }
            out.push(StreamEvent::TextDelta { index, text });
        }
    }

    fn open(&mut self, next_index: &mut usize, out: &mut Vec<StreamEvent>) {
        let index = *next_index;
        *next_index = next_index.saturating_add(1);
        self.index = Some(index);
        out.push(match &self.kind {
            Kind::Text => StreamEvent::BlockStart {
                index,
                kind: BlockKind::Text,
            },
            Kind::Thinking => StreamEvent::BlockStart {
                index,
                kind: BlockKind::Thinking,
            },
            Kind::Redacted => StreamEvent::BlockStart {
                index,
                kind: BlockKind::RedactedThinking,
            },
            Kind::Structured => StreamEvent::BlockStart {
                index,
                kind: BlockKind::Structured,
            },
            Kind::ToolCall { id, name } => StreamEvent::ToolCallStart {
                index,
                id: id.clone(),
                name: name.clone(),
            },
        });
    }
}

fn delta_event(index: usize, raw: Raw) -> Option<StreamEvent> {
    match raw {
        Raw::Thinking(text) => Some(StreamEvent::ThinkingDelta { index, text }),
        Raw::Signature(signature) => Some(StreamEvent::SignatureDelta { index, signature }),
        Raw::Structured(json_fragment) => Some(StreamEvent::StructuredDelta {
            index,
            json_fragment,
        }),
        Raw::Arguments(json_fragment) => Some(StreamEvent::ToolCallArgumentsDelta {
            index,
            json_fragment,
        }),
        Raw::Redacted(data) => Some(StreamEvent::RedactedThinkingData { index, data }),
        Raw::Start | Raw::Text(_) | Raw::End => None,
    }
}
