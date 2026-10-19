use br_llm_clients_common_call::PayloadError;
use serde_json::{Map, Value};

pub(crate) fn wrong(place: &str, expected: &str) -> PayloadError {
    PayloadError::new(format!("the field `{place}` is not {expected}"))
}

pub(crate) fn present<'a>(object: &'a Map<String, Value>, name: &str) -> Option<&'a Value> {
    object.get(name).filter(|value| !value.is_null())
}

pub(crate) fn text_field<'a>(
    object: &'a Map<String, Value>,
    name: &str,
    place: &str,
) -> Result<Option<&'a str>, PayloadError> {
    match present(object, name) {
        None => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.as_str())),
        Some(_) => Err(wrong(place, "a string")),
    }
}

pub(crate) fn object_field<'a>(
    object: &'a Map<String, Value>,
    name: &str,
    place: &str,
) -> Result<Option<&'a Map<String, Value>>, PayloadError> {
    match present(object, name) {
        None => Ok(None),
        Some(Value::Object(members)) => Ok(Some(members)),
        Some(_) => Err(wrong(place, "an object")),
    }
}

pub(crate) fn array_field<'a>(
    object: &'a Map<String, Value>,
    name: &str,
    place: &str,
) -> Result<Option<&'a [Value]>, PayloadError> {
    match present(object, name) {
        None => Ok(None),
        Some(Value::Array(items)) => Ok(Some(items.as_slice())),
        Some(_) => Err(wrong(place, "an array")),
    }
}

pub(crate) fn count_field(
    object: &Map<String, Value>,
    name: &str,
    place: &str,
) -> Result<Option<u64>, PayloadError> {
    match present(object, name) {
        None => Ok(None),
        Some(value) => value
            .as_u64()
            .map(Some)
            .ok_or_else(|| wrong(place, "a non-negative integer")),
    }
}
