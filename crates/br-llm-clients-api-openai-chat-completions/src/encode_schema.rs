use br_llm_clients_core::{Parameter, ParameterKind, Parameters};
use serde_json::{Map, Value, json};

pub(crate) struct UnknownKind {
    pub(crate) parameter: String,
}

pub(crate) fn object_schema(parameters: &Parameters) -> Result<Value, UnknownKind> {
    let mut properties = Map::new();
    let mut required = Vec::new();
    for parameter in parameters {
        properties.insert(parameter.name().to_owned(), parameter_schema(parameter)?);
        if parameter.is_required() {
            required.push(Value::from(parameter.name()));
        }
    }
    let mut schema = Map::new();
    schema.insert("type".to_owned(), Value::from("object"));
    schema.insert("properties".to_owned(), Value::Object(properties));
    if !required.is_empty() {
        schema.insert("required".to_owned(), Value::Array(required));
    }
    Ok(Value::Object(schema))
}

fn parameter_schema(parameter: &Parameter) -> Result<Value, UnknownKind> {
    let mut schema = kind_schema(parameter.name(), parameter.kind())?;
    if !parameter.description().is_empty()
        && let Value::Object(members) = &mut schema
    {
        members.insert(
            "description".to_owned(),
            Value::from(parameter.description()),
        );
    }
    Ok(schema)
}

fn kind_schema(owner: &str, kind: &ParameterKind) -> Result<Value, UnknownKind> {
    match kind {
        ParameterKind::String => Ok(json!({"type": "string"})),
        ParameterKind::Number => Ok(json!({"type": "number"})),
        ParameterKind::Integer => Ok(json!({"type": "integer"})),
        ParameterKind::Boolean => Ok(json!({"type": "boolean"})),
        ParameterKind::Enum { values } => Ok(json!({"type": "string", "enum": values})),
        ParameterKind::Array { items } => {
            Ok(json!({"type": "array", "items": kind_schema(owner, items)?}))
        }
        ParameterKind::Object { parameters } => object_schema(parameters),
        _ => Err(UnknownKind {
            parameter: owner.to_owned(),
        }),
    }
}
