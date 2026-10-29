use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use br_llm_messages::{ReplayFormat, ResponseEnding, Usage};

use crate::error_info::ErrorInfo;
use crate::payload::{Payload, PayloadError};
use crate::payload_decoder::{End, PayloadDecoder};
use crate::provider_event::{ProviderBlock, ProviderEvent};

#[derive(Debug, Clone)]
pub(crate) enum EndScript {
    Events(Vec<String>),
    Missing,
    Failure(String),
}

#[derive(Default)]
pub(crate) struct DecoderLog {
    payloads: Mutex<Vec<Payload>>,
    ends: AtomicUsize,
    created: AtomicUsize,
}

impl DecoderLog {
    pub(crate) fn payloads(&self) -> Vec<Payload> {
        self.payloads.lock().expect("log").clone()
    }

    pub(crate) fn payload_count(&self) -> usize {
        self.payloads.lock().expect("log").len()
    }

    pub(crate) fn ends(&self) -> usize {
        self.ends.load(Ordering::SeqCst)
    }

    pub(crate) fn created(&self) -> usize {
        self.created.load(Ordering::SeqCst)
    }

    pub(crate) fn note_created(&self) {
        self.created.fetch_add(1, Ordering::SeqCst);
    }
}

pub(crate) struct ScriptDecoder {
    pub(crate) log: std::sync::Arc<DecoderLog>,
    pub(crate) end: EndScript,
}

impl PayloadDecoder for ScriptDecoder {
    fn payload(&mut self, payload: Payload) -> Result<Vec<ProviderEvent>, PayloadError> {
        let events = lines(payload.data());
        self.log.payloads.lock().expect("log").push(payload);
        events
    }

    fn end(&mut self) -> Result<End, PayloadError> {
        self.log.ends.fetch_add(1, Ordering::SeqCst);
        match &self.end {
            EndScript::Events(script) => {
                let joined = script.join("\n");
                lines(&joined).map(End::Events)
            }
            EndScript::Missing => Ok(End::MarkerMissing),
            EndScript::Failure(text) => Err(PayloadError::new(text.as_str())),
        }
    }
}

fn lines(data: &str) -> Result<Vec<ProviderEvent>, PayloadError> {
    data.split('\n')
        .filter(|line| !line.is_empty())
        .map(parse)
        .collect()
}

fn bad(line: &str) -> PayloadError {
    PayloadError::new(format!("cannot read the line {line}"))
}

fn optional<T>(text: &str, read: impl Fn(&str) -> Option<T>) -> Option<T> {
    if text == "-" { None } else { read(text) }
}

fn ending(text: &str) -> Option<ResponseEnding> {
    Some(match text {
        "finished" => ResponseEnding::finished(),
        "tool_use" => ResponseEnding::tool_use(),
        "cont" => ResponseEnding::provider_continuation(),
        "exhaust" => ResponseEnding::output_exhaustion(),
        "seq" => ResponseEnding::stop_sequence(),
        "refusal" => ResponseEnding::refusal(),
        "filter" => ResponseEnding::filtering(),
        other => {
            if let Some(sequence) = other.strip_prefix("matched:") {
                ResponseEnding::matched_stop_sequence(sequence).ok()?
            } else {
                ResponseEnding::unclassified(other.strip_prefix("other:")?).ok()?
            }
        }
    })
}

fn block(spec: &str) -> Option<ProviderBlock> {
    let mut words = spec.split(' ');
    Some(match words.next()? {
        "text" => ProviderBlock::Text,
        "thinking" => ProviderBlock::Thinking,
        "redacted" => ProviderBlock::RedactedThinking,
        "structured" => ProviderBlock::Structured,
        "tool" => ProviderBlock::ToolCall {
            name: words.next()?.to_owned(),
            id: words
                .next()
                .map(|id| if id == "~" { "" } else { id }.to_owned()),
        },
        _ => return None,
    })
}

fn error(line: &str) -> Option<ProviderEvent> {
    let mut parts = line.splitn(5, ' ').skip(1);
    let status = optional(parts.next()?, |text| text.parse::<u16>().ok());
    let code = optional(parts.next()?, |text| Some(text.to_owned()));
    let delay = optional(parts.next()?, |text| text.parse::<u64>().ok());
    let mut info = ErrorInfo::new(parts.next().unwrap_or_default());
    if let Some(status) = status {
        info = info.with_status(status);
    }
    if let Some(code) = code {
        info = info.with_code(code);
    }
    if let Some(delay) = delay {
        info = info.with_retry_after(Duration::from_secs(delay));
    }
    Some(ProviderEvent::Error(info))
}

fn usage(line: &str) -> Option<ProviderEvent> {
    let mut parts = line.split(' ').skip(1);
    let input = parts.next()?.parse().ok()?;
    let usage = Usage::of_input_tokens(input);
    Some(ProviderEvent::Usage(match parts.next() {
        Some(output) => usage.with_output_tokens(output.parse().ok()?),
        None => usage,
    }))
}

fn format_and_text(rest: &str) -> Option<(ReplayFormat, String)> {
    let (format, text) = rest.split_once(' ').unwrap_or((rest, ""));
    Some((ReplayFormat::new(format).ok()?, text.to_owned()))
}

fn parse(line: &str) -> Result<ProviderEvent, PayloadError> {
    read(line).ok_or_else(|| bad(line))
}

fn read(line: &str) -> Option<ProviderEvent> {
    let mut parts = line.splitn(3, ' ');
    let command = parts.next()?;
    let key = parts.next().unwrap_or_default().to_owned();
    let rest = parts.next().unwrap_or_default().to_owned();
    Some(match command {
        "start" => ProviderEvent::BlockStart {
            key,
            block: block(&rest)?,
        },
        "end" => ProviderEvent::BlockEnd { key },
        "text" => ProviderEvent::TextDelta { key, text: rest },
        "think" => ProviderEvent::ThinkingDelta { key, text: rest },
        "sig" => {
            let (format, signature) = format_and_text(&rest)?;
            ProviderEvent::SignatureDelta {
                key,
                format,
                signature,
            }
        }
        "struct" => ProviderEvent::StructuredDelta { key, json: rest },
        "args" => ProviderEvent::ToolArgumentsDelta { key, json: rest },
        "redact" => {
            let (format, data) = format_and_text(&rest)?;
            ProviderEvent::RedactedThinkingData { key, format, data }
        }
        "stop" => ProviderEvent::Stop(ending(&key)?),
        "rid" => ProviderEvent::ResponseId(key),
        "usage" => return usage(line),
        "model" => ProviderEvent::Model(key),
        "error" => return error(line),
        "complete" => ProviderEvent::Complete,
        _ => return None,
    })
}

pub(crate) fn script(script: &[&str]) -> Vec<ProviderEvent> {
    script
        .iter()
        .map(|line| parse(line).expect("a line of the test grammar"))
        .collect()
}
