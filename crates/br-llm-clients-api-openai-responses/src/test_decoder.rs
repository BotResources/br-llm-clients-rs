use br_llm_clients_common_call::{
    Dialect, End, Payload, PayloadDecoder, PayloadError, ProviderBlock, ProviderEvent,
};
use br_llm_clients_core::{OutputMode, ReasoningLevel};
use br_llm_messages::{StopReason, Usage};
use serde_json::Value;

use crate::test_requests::{dialect, parameter, shape};
use br_llm_clients_core::ParameterKind;

pub(crate) fn structured_mode() -> OutputMode {
    OutputMode::Structured(shape(
        "answer",
        None,
        vec![parameter("value", "", true, ParameterKind::String)],
    ))
}

pub(crate) fn plain_decoder() -> Box<dyn PayloadDecoder> {
    dialect(ReasoningLevel::Off).decoder(&OutputMode::Plain)
}

pub(crate) fn structured_decoder() -> Box<dyn PayloadDecoder> {
    dialect(ReasoningLevel::Off).decoder(&structured_mode())
}

pub(crate) fn feed(decoder: &mut Box<dyn PayloadDecoder>, event: &Value) -> Vec<ProviderEvent> {
    decoder
        .payload(Payload::new(None, event.to_string()))
        .expect("the payload is readable")
}

pub(crate) fn feed_err(decoder: &mut Box<dyn PayloadDecoder>, event: &Value) -> PayloadError {
    decoder
        .payload(Payload::new(None, event.to_string()))
        .expect_err("a payload error")
}

pub(crate) fn run_all(
    decoder: &mut Box<dyn PayloadDecoder>,
    events: &[Value],
) -> Vec<Vec<ProviderEvent>> {
    events.iter().map(|event| feed(decoder, event)).collect()
}

pub(crate) fn flat(decoder: &mut Box<dyn PayloadDecoder>, events: &[Value]) -> Vec<ProviderEvent> {
    run_all(decoder, events).into_iter().flatten().collect()
}

pub(crate) fn end_of(decoder: &mut Box<dyn PayloadDecoder>) -> Result<End, PayloadError> {
    decoder.end()
}

pub(crate) fn start_text(key: &str) -> ProviderEvent {
    ProviderEvent::BlockStart {
        key: key.to_owned(),
        block: ProviderBlock::Text,
    }
}

pub(crate) fn start_structured(key: &str) -> ProviderEvent {
    ProviderEvent::BlockStart {
        key: key.to_owned(),
        block: ProviderBlock::Structured,
    }
}

pub(crate) fn start_thinking(key: &str) -> ProviderEvent {
    ProviderEvent::BlockStart {
        key: key.to_owned(),
        block: ProviderBlock::Thinking,
    }
}

pub(crate) fn start_call(key: &str, name: &str, id: Option<&str>) -> ProviderEvent {
    ProviderEvent::BlockStart {
        key: key.to_owned(),
        block: ProviderBlock::ToolCall {
            name: name.to_owned(),
            id: id.map(str::to_owned),
        },
    }
}

pub(crate) fn text(key: &str, value: &str) -> ProviderEvent {
    ProviderEvent::TextDelta {
        key: key.to_owned(),
        text: value.to_owned(),
    }
}

pub(crate) fn thinking(key: &str, value: &str) -> ProviderEvent {
    ProviderEvent::ThinkingDelta {
        key: key.to_owned(),
        text: value.to_owned(),
    }
}

pub(crate) fn signature(key: &str, value: &str) -> ProviderEvent {
    ProviderEvent::SignatureDelta {
        key: key.to_owned(),
        signature: value.to_owned(),
    }
}

pub(crate) fn structured(key: &str, value: &str) -> ProviderEvent {
    ProviderEvent::StructuredDelta {
        key: key.to_owned(),
        json: value.to_owned(),
    }
}

pub(crate) fn arguments(key: &str, value: &str) -> ProviderEvent {
    ProviderEvent::ToolArgumentsDelta {
        key: key.to_owned(),
        json: value.to_owned(),
    }
}

pub(crate) fn end(key: &str) -> ProviderEvent {
    ProviderEvent::BlockEnd {
        key: key.to_owned(),
    }
}

pub(crate) fn stop(reason: StopReason) -> ProviderEvent {
    ProviderEvent::Stop(reason)
}

pub(crate) fn usage(input: u64, output: u64) -> ProviderEvent {
    ProviderEvent::Usage(Usage {
        input_tokens: input,
        output_tokens: output,
        cache_read_tokens: Some(0),
        cache_write_tokens: None,
        thinking_tokens: Some(0),
    })
}

pub(crate) fn model(value: &str) -> ProviderEvent {
    ProviderEvent::Model(value.to_owned())
}
