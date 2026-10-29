use br_llm_clients_common_call::{
    Dialect, End, Payload, PayloadDecoder, PayloadError, ProviderBlock, ProviderEvent,
};
use br_llm_clients_core::OutputMode;
use br_llm_messages::{ReplayFormat, ResponseEnding, Usage};
use serde_json::Value;

use crate::replay_formats::{REDACTED_DATA_FORMAT, SIGNATURE_FORMAT};
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
        .and_then(|value| value.get("type").and_then(Value::as_str).map(str::to_owned))
        .filter(|kind| kind != "error");
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
        format: ReplayFormat::new(SIGNATURE_FORMAT).expect("a format"),
        signature: signature.to_owned(),
    }
}

pub(crate) fn redacted_data(index: &str, data: &str) -> ProviderEvent {
    ProviderEvent::RedactedThinkingData {
        key: key(index),
        format: ReplayFormat::new(REDACTED_DATA_FORMAT).expect("a format"),
        data: data.to_owned(),
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
    Usage::of_input_tokens(input).with_output_tokens(output)
}

pub(crate) fn stop(ending: ResponseEnding) -> ProviderEvent {
    ProviderEvent::Stop(ending)
}

pub(crate) fn block_start(index: &str, block: &str) -> String {
    format!(r#"{{"type":"content_block_start","index":{index},"content_block":{block}}}"#)
}

pub(crate) fn delta(index: u64, delta: &str) -> String {
    format!(r#"{{"type":"content_block_delta","index":{index},"delta":{delta}}}"#)
}

pub(crate) fn stop_of(index: u64) -> String {
    format!(r#"{{"type":"content_block_stop","index":{index}}}"#)
}

pub(crate) fn start_with(usage: &str) -> (Session, Vec<ProviderEvent>) {
    let mut session = Session::plain();
    let events = session.ok(&format!(
        r#"{{"type":"message_start","message":{{"usage":{usage}}}}}"#
    ));
    (session, events)
}

pub(crate) fn delta_usage(session: &mut Session, usage: &str) -> Vec<ProviderEvent> {
    session.ok(&format!(
        r#"{{"type":"message_delta","delta":{{}},"usage":{usage}}}"#
    ))
}

pub(crate) fn only_usage(events: Vec<ProviderEvent>) -> Usage {
    match events.as_slice() {
        [ProviderEvent::Usage(usage)] => usage.clone(),
        other => panic!("expected one usage event, got {other:?}"),
    }
}
