use br_llm_clients_core::{ClientError, OutputMode, StructuredShape};
use serde_json::{Map, Value};

use crate::encode_schema::object_schema;

const MAX_NAME_LENGTH: usize = 64;

pub(crate) fn response_format(output: &OutputMode) -> Result<Option<Value>, ClientError> {
    match output {
        OutputMode::Plain => Ok(None),
        OutputMode::Structured(shape) => structured(shape).map(Some),
    }
}

fn structured(shape: &StructuredShape) -> Result<Value, ClientError> {
    check_name(shape.name())?;
    let schema = object_schema(shape.parameters()).map_err(|unknown| {
        rejected(format!(
            "the parameter `{}` has a kind that this dialect does not know",
            unknown.parameter
        ))
    })?;
    let mut json_schema = Map::new();
    json_schema.insert("name".to_owned(), Value::from(shape.name()));
    if let Some(description) = shape.description().filter(|text| !text.is_empty()) {
        json_schema.insert("description".to_owned(), Value::from(description));
    }
    json_schema.insert("schema".to_owned(), schema);
    let mut format = Map::new();
    format.insert("type".to_owned(), Value::from("json_schema"));
    format.insert("json_schema".to_owned(), Value::Object(json_schema));
    Ok(Value::Object(format))
}

fn check_name(name: &str) -> Result<(), ClientError> {
    let allowed = |character: char| character.is_ascii_alphanumeric() || "_-".contains(character);
    let length = name.chars().count();
    if length == 0 || length > MAX_NAME_LENGTH || !name.chars().all(allowed) {
        return Err(rejected(
            "the name of the shape must be 1 to 64 characters from A-Z, a-z, 0-9, _ and -"
                .to_owned(),
        ));
    }
    Ok(())
}

fn rejected(reason: String) -> ClientError {
    ClientError::OutputShapeRejected { reason }
}
