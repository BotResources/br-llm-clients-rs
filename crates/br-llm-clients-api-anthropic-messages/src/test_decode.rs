use br_llm_clients_common_call::{
    Dialect, End, Payload, PayloadDecoder, PayloadError, ProviderBlock, ProviderEvent,
};
use br_llm_clients_core::OutputMode;
use br_llm_messages::{StopReason, Usage};
use serde_json::Value;

use crate::test_fixtures::{dialect, structured};

pub(crate) fn decoder(structured_mode: bool) -> Box<dyn PayloadDecoder> {
    let mode = if structured_mode {
        structured(None, vec![])
    } else {
        OutputMode::Plain
    };
    dialect().decoder(&mode)
}

pub(crate) fn payloads(stream: &str) -> Vec<Payload> {
    stream
        .split("\n\n")
        .filter(|frame| !frame.trim().is_empty())
        .map(|frame| {
            let mut event = None;
            let mut data = Vec::new();
            for line in frame.lines() {
                if let Some(name) = line.strip_prefix("event: ") {
                    event = Some(name.to_owned());
                } else if let Some(text) = line.strip_prefix("data: ") {
                    data.push(text);
                }
            }
            Payload::new(event, data.join("\n"))
        })
        .collect()
}

pub(crate) fn data(json: &str) -> Payload {
    let kind = serde_json::from_str::<Value>(json)
        .ok()
        .and_then(|value| value.get("type").and_then(Value::as_str).map(str::to_owned));
    Payload::new(kind, json)
}

pub(crate) fn decode_stream(structured_mode: bool, stream: &str) -> Vec<ProviderEvent> {
    let mut decoder = decoder(structured_mode);
    payloads(stream)
        .into_iter()
        .flat_map(|payload| decoder.payload(payload).expect("a readable payload"))
        .collect()
}

pub(crate) struct Session {
    decoder: Box<dyn PayloadDecoder>,
}

impl Session {
    pub(crate) fn plain() -> Self {
        Self {
            decoder: decoder(false),
        }
    }

    pub(crate) fn structured() -> Self {
        Self {
            decoder: decoder(true),
        }
    }

    pub(crate) fn started() -> Self {
        let mut session = Self::plain();
        session.ok(r#"{"type":"message_start","message":{"model":"m"}}"#);
        session
    }

    pub(crate) fn feed(&mut self, json: &str) -> Result<Vec<ProviderEvent>, PayloadError> {
        self.decoder.payload(data(json))
    }

    pub(crate) fn ok(&mut self, json: &str) -> Vec<ProviderEvent> {
        self.feed(json).expect("a readable payload")
    }

    pub(crate) fn rejected(&mut self, json: &str) {
        assert!(self.feed(json).is_err(), "{json} was accepted");
    }

    pub(crate) fn end(&mut self) -> Result<End, PayloadError> {
        self.decoder.end()
    }
}

pub(crate) fn key(value: &str) -> String {
    value.to_owned()
}

pub(crate) fn start(index: &str, block: ProviderBlock) -> ProviderEvent {
    ProviderEvent::BlockStart {
        key: key(index),
        block,
    }
}

pub(crate) fn tool_start(index: &str, name: &str, id: &str) -> ProviderEvent {
    start(
        index,
        ProviderBlock::ToolCall {
            name: name.to_owned(),
            id: Some(id.to_owned()),
        },
    )
}

pub(crate) fn text_delta(index: &str, text: &str) -> ProviderEvent {
    ProviderEvent::TextDelta {
        key: key(index),
        text: text.to_owned(),
    }
}

pub(crate) fn thinking_delta(index: &str, text: &str) -> ProviderEvent {
    ProviderEvent::ThinkingDelta {
        key: key(index),
        text: text.to_owned(),
    }
}

pub(crate) fn signature_delta(index: &str, signature: &str) -> ProviderEvent {
    ProviderEvent::SignatureDelta {
        key: key(index),
        signature: signature.to_owned(),
    }
}

pub(crate) fn args_delta(index: &str, json: &str) -> ProviderEvent {
    ProviderEvent::ToolArgumentsDelta {
        key: key(index),
        json: json.to_owned(),
    }
}

pub(crate) fn end_of(index: &str) -> ProviderEvent {
    ProviderEvent::BlockEnd { key: key(index) }
}

pub(crate) fn usage(input: u64, output: u64) -> Usage {
    Usage {
        input_tokens: input,
        output_tokens: output,
        cache_read_tokens: None,
        cache_write_tokens: None,
        thinking_tokens: None,
    }
}

pub(crate) fn stop(reason: StopReason) -> ProviderEvent {
    ProviderEvent::Stop(reason)
}
