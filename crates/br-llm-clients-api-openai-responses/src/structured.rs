use br_llm_clients_core::{ClientError, LimitKind, StructuredShape};
use serde_json::{Map, Value};

use crate::schema::{is_strict, object_schema};

const MAX_NAME_CHARACTERS: usize = 64;

fn is_allowed(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_' || character == '-'
}

fn check_name(name: &str) -> Result<(), ClientError> {
    if !name.chars().all(is_allowed) {
        return Err(ClientError::OutputShapeRejected {
            reason: "the name of the shape holds a character outside A-Z, a-z, 0-9, _ and -"
                .to_owned(),
        });
    }
    if name.chars().count() > MAX_NAME_CHARACTERS {
        return Err(ClientError::LimitExceeded {
            kind: LimitKind::Other,
            description: format!(
                "the name of the shape is longer than {MAX_NAME_CHARACTERS} characters"
            ),
        });
    }
    Ok(())
}

pub(crate) fn text_field(shape: &StructuredShape) -> Result<Value, ClientError> {
    check_name(shape.name())?;
    let schema =
        object_schema(shape.parameters()).map_err(|_| ClientError::OutputShapeRejected {
            reason: "a parameter has a kind that this dialect does not know".to_owned(),
        })?;
    let mut format = Map::new();
    format.insert("type".to_owned(), Value::String("json_schema".to_owned()));
    format.insert("name".to_owned(), Value::String(shape.name().to_owned()));
    if let Some(description) = shape.description() {
        format.insert(
            "description".to_owned(),
            Value::String(description.to_owned()),
        );
    }
    format.insert("schema".to_owned(), schema);
    format.insert(
        "strict".to_owned(),
        Value::Bool(is_strict(shape.parameters())),
    );
    let mut text = Map::new();
    text.insert("format".to_owned(), Value::Object(format));
    Ok(Value::Object(text))
}
