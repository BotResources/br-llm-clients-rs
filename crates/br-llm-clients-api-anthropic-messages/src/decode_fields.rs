use br_llm_clients_common_call::PayloadError;
use serde_json::{Map, Value};

pub(crate) type Fields = Map<String, Value>;

pub(crate) fn malformed(text: impl Into<String>) -> PayloadError {
    PayloadError::new(text)
}

pub(crate) fn index_of(fields: &Fields) -> Result<u64, PayloadError> {
    fields
        .get("index")
        .and_then(Value::as_u64)
        .ok_or_else(|| malformed("the index is missing or is not a non-negative integer"))
}

pub(crate) fn object_field<'a>(fields: &'a Fields, name: &str) -> Result<&'a Fields, PayloadError> {
    fields
        .get(name)
        .and_then(Value::as_object)
        .ok_or_else(|| malformed(format!("the field {name} is missing or is not an object")))
}

pub(crate) fn type_of<'a>(fields: &'a Fields, owner: &str) -> Result<&'a str, PayloadError> {
    fields
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| malformed(format!("the type of the {owner} is missing or is not text")))
}

pub(crate) fn required_text<'a>(fields: &'a Fields, name: &str) -> Result<&'a str, PayloadError> {
    fields
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| malformed(format!("the field {name} is missing or is not text")))
}

pub(crate) fn optional_text<'a>(
    fields: &'a Fields,
    name: &str,
) -> Result<Option<&'a str>, PayloadError> {
    match fields.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.as_str())),
        Some(_) => Err(malformed(format!("the field {name} is not text"))),
    }
}

pub(crate) fn non_empty_text<'a>(fields: &'a Fields, name: &str) -> Result<&'a str, PayloadError> {
    match fields.get(name).and_then(Value::as_str) {
        Some(text) if !text.is_empty() => Ok(text),
        _ => Err(malformed(format!(
            "the field {name} is missing, is empty or is not text"
        ))),
    }
}
