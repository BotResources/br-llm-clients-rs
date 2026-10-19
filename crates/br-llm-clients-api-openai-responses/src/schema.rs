use br_llm_clients_core::{Parameter, ParameterKind, Parameters};
use serde_json::{Map, Value, json};

pub(crate) struct UnknownKind;

pub(crate) fn object_schema(parameters: &Parameters) -> Result<Value, UnknownKind> {
    let mut properties = Map::new();
    let mut required = Vec::new();
    for parameter in parameters {
        properties.insert(parameter.name().to_owned(), property(parameter)?);
        if parameter.is_required() {
            required.push(Value::String(parameter.name().to_owned()));
        }
    }
    Ok(json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false,
    }))
}

fn property(parameter: &Parameter) -> Result<Value, UnknownKind> {
    let mut schema = kind_schema(parameter.kind())?;
    if !parameter.description().is_empty()
        && let Value::Object(fields) = &mut schema
    {
        fields.insert(
            "description".to_owned(),
            Value::String(parameter.description().to_owned()),
        );
    }
    Ok(schema)
}

fn kind_schema(kind: &ParameterKind) -> Result<Value, UnknownKind> {
    Ok(match kind {
        ParameterKind::String => json!({ "type": "string" }),
        ParameterKind::Number => json!({ "type": "number" }),
        ParameterKind::Integer => json!({ "type": "integer" }),
        ParameterKind::Boolean => json!({ "type": "boolean" }),
        ParameterKind::Enum { values } => json!({ "type": "string", "enum": values }),
        ParameterKind::Array { items } => {
            json!({ "type": "array", "items": kind_schema(items)? })
        }
        ParameterKind::Object { parameters } => object_schema(parameters)?,
        _ => return Err(UnknownKind),
    })
}

pub(crate) fn is_strict(parameters: &Parameters) -> bool {
    parameters
        .iter()
        .all(|parameter| parameter.is_required() && kind_is_strict(parameter.kind()))
}

fn kind_is_strict(kind: &ParameterKind) -> bool {
    match kind {
        ParameterKind::Array { items } => kind_is_strict(items),
        ParameterKind::Object { parameters } => is_strict(parameters),
        _ => true,
    }
}
