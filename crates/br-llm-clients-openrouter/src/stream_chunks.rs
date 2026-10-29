use br_llm_clients_common_call::{End, Payload, PayloadDecoder, PayloadError, ProviderEvent};
use serde_json::{Map, Value};

const FINISH_REASON: &str = "finish_reason";
const ERROR: &str = "error";

pub(crate) struct StreamChunks {
    inner: Box<dyn PayloadDecoder>,
    first_finish: Option<String>,
}

enum Rewrite {
    Unchanged,
    Chunk,
    ChunkThenError(Value),
}

impl StreamChunks {
    pub(crate) fn new(inner: Box<dyn PayloadDecoder>) -> Self {
        Self {
            inner,
            first_finish: None,
        }
    }

    fn normalized(&mut self, payload: Payload) -> Vec<Payload> {
        let Ok(Value::Object(mut chunk)) = serde_json::from_str::<Value>(payload.data()) else {
            return vec![payload];
        };
        let event = payload.event().map(str::to_owned);
        match self.rewrite(&mut chunk) {
            Rewrite::Unchanged => vec![payload],
            Rewrite::Chunk => vec![Payload::new(event, Value::Object(chunk).to_string())],
            Rewrite::ChunkThenError(error) => {
                let mut alone = Map::new();
                alone.insert(ERROR.to_owned(), error);
                vec![
                    Payload::new(event.clone(), Value::Object(chunk).to_string()),
                    Payload::new(event, Value::Object(alone).to_string()),
                ]
            }
        }
    }

    fn rewrite(&mut self, chunk: &mut Map<String, Value>) -> Rewrite {
        if is_present(chunk.get(ERROR)) {
            return Rewrite::Unchanged;
        }
        let Some(choice) = only_choice(chunk) else {
            return Rewrite::Unchanged;
        };
        if is_present(choice.get(ERROR)) {
            choice.remove(FINISH_REASON);
            return match choice.remove(ERROR) {
                Some(error) => Rewrite::ChunkThenError(error),
                None => Rewrite::Unchanged,
            };
        }
        let reason = match choice.get(FINISH_REASON) {
            Some(Value::String(reason)) if !reason.is_empty() => reason.clone(),
            _ => return Rewrite::Unchanged,
        };
        match &self.first_finish {
            None => {
                self.first_finish = Some(reason);
                Rewrite::Unchanged
            }
            Some(first) if *first == reason => {
                choice.remove(FINISH_REASON);
                Rewrite::Chunk
            }
            Some(_) => Rewrite::Unchanged,
        }
    }
}

fn is_present(value: Option<&Value>) -> bool {
    value.is_some_and(|value| !value.is_null())
}

fn only_choice(chunk: &mut Map<String, Value>) -> Option<&mut Map<String, Value>> {
    match chunk.get_mut("choices") {
        Some(Value::Array(choices)) => match choices.as_mut_slice() {
            [Value::Object(choice)] => Some(choice),
            _ => None,
        },
        _ => None,
    }
}

impl PayloadDecoder for StreamChunks {
    fn payload(&mut self, payload: Payload) -> Result<Vec<ProviderEvent>, PayloadError> {
        let mut events = Vec::new();
        for payload in self.normalized(payload) {
            events.extend(self.inner.payload(payload)?);
        }
        Ok(events)
    }

    fn end(&mut self) -> Result<End, PayloadError> {
        self.inner.end()
    }
}
