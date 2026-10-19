use std::collections::BTreeMap;

use br_llm_clients_common_call::PayloadError;
use serde_json::{Map, Value};

use crate::json_fields::{object_field, present, text_field, wrong};

const MAX_INDEX: u64 = 1023;
const FUNCTION_TYPE: &str = "function";

#[derive(Debug, Default)]
pub(crate) struct HeldCall {
    pub(crate) id: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) arguments: String,
}

pub(crate) type HeldCalls = BTreeMap<u16, HeldCall>;

pub(crate) fn hold(held: &mut HeldCalls, fragments: &[Value]) -> Result<(), PayloadError> {
    for fragment in fragments {
        let Value::Object(fragment) = fragment else {
            return Err(wrong("tool_calls[]", "an object"));
        };
        hold_one(held, fragment)?;
    }
    Ok(())
}

fn hold_one(held: &mut HeldCalls, fragment: &Map<String, Value>) -> Result<(), PayloadError> {
    let index = index_of(fragment)?;
    check_type(fragment)?;
    let id = non_empty(text_field(fragment, "id", "tool_calls[].id")?);
    let function = object_field(fragment, "function", "tool_calls[].function")?;
    let name = match function {
        Some(function) => non_empty(text_field(function, "name", "tool_calls[].function.name")?),
        None => None,
    };
    let arguments = match function {
        Some(function) => text_field(function, "arguments", "tool_calls[].function.arguments")?,
        None => None,
    };
    let call = held.entry(index).or_default();
    merge(&mut call.id, id, "tool_calls[].id")?;
    merge(&mut call.name, name, "tool_calls[].function.name")?;
    call.arguments.push_str(arguments.unwrap_or_default());
    Ok(())
}

fn index_of(fragment: &Map<String, Value>) -> Result<u16, PayloadError> {
    present(fragment, "index")
        .and_then(Value::as_u64)
        .filter(|index| *index <= MAX_INDEX)
        .and_then(|index| u16::try_from(index).ok())
        .ok_or_else(|| wrong("tool_calls[].index", "an integer from 0 to 1023"))
}

fn check_type(fragment: &Map<String, Value>) -> Result<(), PayloadError> {
    match present(fragment, "type") {
        None => Ok(()),
        Some(Value::String(kind)) if kind == FUNCTION_TYPE => Ok(()),
        Some(_) => Err(PayloadError::new(
            "a tool call has a type other than `function`",
        )),
    }
}

fn non_empty(text: Option<&str>) -> Option<&str> {
    text.filter(|text| !text.is_empty())
}

fn merge(
    held: &mut Option<String>,
    incoming: Option<&str>,
    place: &str,
) -> Result<(), PayloadError> {
    match (held.as_deref(), incoming) {
        (_, None) => Ok(()),
        (None, Some(value)) => {
            *held = Some(value.to_owned());
            Ok(())
        }
        (Some(current), Some(value)) if current == value => Ok(()),
        (Some(_), Some(_)) => Err(PayloadError::new(format!(
            "the field `{place}` changes between the fragments of one tool call"
        ))),
    }
}
