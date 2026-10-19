use br_llm_clients_common_call::{End, Payload, PayloadDecoder, PayloadError, ProviderEvent};
use br_llm_clients_core::OutputMode;
use serde_json::{Map, Value};

use crate::choice::single_choice;
use crate::json_fields::text_field;
use crate::options::ChatCompletionsOptions;
use crate::stream_error::error_info;
use crate::tool_fragments::HeldCalls;
use crate::usage;

const SENTINEL: &str = "[DONE]";

pub(crate) struct ChatCompletionsDecoder {
    pub(crate) structured: bool,
    pub(crate) reasoning_field: Option<String>,
    cache_read_field: Option<String>,
    pub(crate) open: Option<&'static str>,
    pub(crate) held: HeldCalls,
    pub(crate) finished: bool,
    pub(crate) refusal_seen: bool,
    model: Option<String>,
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
        }
    }

    fn chunk(&mut self, chunk: &Map<String, Value>) -> Result<Vec<ProviderEvent>, PayloadError> {
        if let Some(error) = chunk.get("error").filter(|error| !error.is_null()) {
            return Ok(vec![ProviderEvent::Error(error_info(error))]);
        }
        let mut events = Vec::new();
        if let Some(model) = text_field(chunk, "model", "model")?
            && !model.is_empty()
            && self.model.as_deref() != Some(model)
        {
            self.model = Some(model.to_owned());
            events.push(ProviderEvent::Model(model.to_owned()));
        }
        if let Some(choice) = single_choice(chunk)? {
            self.read_delta(&choice.delta, &mut events)?;
            self.read_finish(choice.finish_reason.as_deref(), &mut events)?;
        }
        if let Some(usage) = usage::read(chunk, self.cache_read_field.as_deref())? {
            events.push(ProviderEvent::Usage(usage));
        }
        Ok(events)
    }
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
