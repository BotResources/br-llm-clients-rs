use br_llm_clients_core::{Parameter, ParameterKind, Parameters};
use serde_json::{Map, Value, json};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Closure {
    Open,
    Closed,
}

pub(crate) fn object_schema(
    parameters: &Parameters,
    closure: Closure,
) -> Result<Map<String, Value>, String> {
    let mut properties = Map::new();
    let mut required = Vec::new();
    for parameter in parameters {
        properties.insert(
            parameter.name().to_owned(),
            Value::Object(property(parameter, closure)?),
        );
        if parameter.is_required() {
            required.push(Value::String(parameter.name().to_owned()));
        }
    }
    let mut schema = Map::new();
    schema.insert("type".to_owned(), json!("object"));
    schema.insert("properties".to_owned(), Value::Object(properties));
    if !required.is_empty() {
        schema.insert("required".to_owned(), Value::Array(required));
    }
    if closure == Closure::Closed {
        schema.insert("additionalProperties".to_owned(), Value::Bool(false));
    }
    Ok(schema)
}

fn property(parameter: &Parameter, closure: Closure) -> Result<Map<String, Value>, String> {
    let mut property = kind_schema(parameter.name(), parameter.kind(), closure)?;
    if !parameter.description().is_empty() {
        property.insert("description".to_owned(), json!(parameter.description()));
    }
    Ok(property)
}

fn kind_schema(
    owner: &str,
    kind: &ParameterKind,
    closure: Closure,
) -> Result<Map<String, Value>, String> {
    let typed = |name: &str| {
        let mut schema = Map::new();
        schema.insert("type".to_owned(), json!(name));
        schema
    };
    match kind {
        ParameterKind::String => Ok(typed("string")),
        ParameterKind::Number => Ok(typed("number")),
        ParameterKind::Integer => Ok(typed("integer")),
        ParameterKind::Boolean => Ok(typed("boolean")),
        ParameterKind::Enum { values } => {
            let mut schema = typed("string");
            schema.insert("enum".to_owned(), json!(values));
            Ok(schema)
        }
        ParameterKind::Array { items } => {
            let mut schema = typed("array");
            schema.insert(
                "items".to_owned(),
                Value::Object(kind_schema(owner, items, closure)?),
            );
            Ok(schema)
        }
        ParameterKind::Object { parameters } => object_schema(parameters, closure),
        _ => Err(format!(
            "parameter {owner:?} has a kind that this version does not know"
        )),
    }
}
