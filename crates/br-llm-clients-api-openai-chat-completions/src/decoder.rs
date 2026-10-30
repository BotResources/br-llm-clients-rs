use br_llm_clients_common_call::{End, Payload, PayloadDecoder, PayloadError, ProviderEvent};
use br_llm_clients_core::OutputMode;
use serde_json::{Map, Value};

use crate::choice::single_choice;
use crate::json_fields::text_field;
use crate::options::ChatCompletionsOptions;
use crate::stream_error::error_info;
use crate::tool_fragments::HeldCalls;
use crate::usage;

mod delta;
mod finish;

const SENTINEL: &str = "[DONE]";

pub(crate) struct ChatCompletionsDecoder {
    structured: bool,
    reasoning_field: Option<String>,
    cache_read_field: Option<String>,
    open: Option<&'static str>,
    held: HeldCalls,
    finished: bool,
    refusal_seen: bool,
    model: Option<String>,
    response_id: Option<String>,
}

impl ChatCompletionsDecoder {
    pub(crate) fn new(output: &OutputMode, options: &ChatCompletionsOptions) -> Self {
        Self {
            structured: matches!(output, OutputMode::Structured(_)),
            reasoning_field: options.reasoning_field().map(str::to_owned),
            cache_read_field: options.cache_read_field().map(str::to_owned),
            open: None,
            held: HeldCalls::new(),
            finished: false,
            refusal_seen: false,
            model: None,
            response_id: None,
        }
    }

    fn chunk(&mut self, chunk: &Map<String, Value>) -> Result<Vec<ProviderEvent>, PayloadError> {
        if let Some(error) = chunk.get("error").filter(|error| !error.is_null()) {
            return Ok(vec![ProviderEvent::Error(error_info(error))]);
        }
        let mut events = Vec::new();
        if let Some(id) = changed(&mut self.response_id, text_field(chunk, "id", "id")?) {
            events.push(ProviderEvent::ResponseId(id));
        }
        if let Some(model) = changed(&mut self.model, text_field(chunk, "model", "model")?) {
            events.push(ProviderEvent::Model(model));
        }
        if let Some(choice) = single_choice(chunk)? {
            let empty = Map::new();
            self.read_delta(choice.delta.unwrap_or(&empty), &mut events)?;
            self.read_finish(choice.finish, &mut events)?;
        }
        if let Some(usage) = usage::read(chunk, self.cache_read_field.as_deref())? {
            events.push(ProviderEvent::Usage(usage));
        }
        Ok(events)
    }
}

fn changed(last: &mut Option<String>, value: Option<&str>) -> Option<String> {
    let value = value.filter(|value| !value.is_empty())?;
    if last.as_deref() == Some(value) {
        return None;
    }
    *last = Some(value.to_owned());
    Some(value.to_owned())
}

impl PayloadDecoder for ChatCompletionsDecoder {
    fn payload(&mut self, payload: Payload) -> Result<Vec<ProviderEvent>, PayloadError> {
        let data = payload.data();
        if data.trim().is_empty() {
            return Ok(Vec::new());
        }
        if data == SENTINEL {
            return Ok(vec![ProviderEvent::Complete]);
        }
        match serde_json::from_str::<Value>(data) {
            Ok(Value::Object(chunk)) => self.chunk(&chunk),
            Ok(_) | Err(_) => Err(PayloadError::new(
                "the data of a payload is neither the end sentinel nor a JSON object",
            )),
        }
    }

    fn end(&mut self) -> Result<End, PayloadError> {
        Ok(End::MarkerMissing)
    }
}
