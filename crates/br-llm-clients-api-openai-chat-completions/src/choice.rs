use br_llm_clients_common_call::PayloadError;
use serde_json::{Map, Value};

use crate::ending::Finish;
use crate::json_fields::{array_field, object_field, present, text_field, wrong};

pub(crate) struct Choice<'a> {
    pub(crate) delta: Option<&'a Map<String, Value>>,
    pub(crate) finish: Option<Finish>,
}

pub(crate) fn single_choice(
    chunk: &Map<String, Value>,
) -> Result<Option<Choice<'_>>, PayloadError> {
    let choices = array_field(chunk, "choices", "choices")?.unwrap_or_default();
    let choice = match choices {
        [] => return Ok(None),
        [Value::Object(choice)] => choice,
        [_] => return Err(wrong("choices[]", "an object")),
        _ => {
            return Err(PayloadError::new(
                "the field `choices` holds more than one choice",
            ));
        }
    };
    check_index(choice)?;
    let delta = object_field(choice, "delta", "choices[].delta")?;
    let finish =
        text_field(choice, "finish_reason", "choices[].finish_reason")?.and_then(Finish::of_reason);
    Ok(Some(Choice { delta, finish }))
}

fn check_index(choice: &Map<String, Value>) -> Result<(), PayloadError> {
    match present(choice, "index") {
        None => Ok(()),
        Some(index) if index.as_u64() == Some(0) => Ok(()),
        Some(_) => Err(wrong("choices[].index", "0")),
    }
}
