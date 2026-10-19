use br_llm_clients_core::{ClientError, OutputMode, ToolSpec};
use serde_json::{Map, Value, json};

use crate::encode_schema::{Closure, object_schema};

pub(crate) fn tool_values(tools: &[ToolSpec]) -> Result<Vec<Value>, ClientError> {
    tools.iter().map(tool_value).collect()
}

fn tool_value(tool: &ToolSpec) -> Result<Value, ClientError> {
    let schema = object_schema(tool.parameters(), Closure::Open).map_err(|reason| {
        ClientError::ToolSchemaRejected {
            tool: Some(tool.name().clone()),
            reason,
        }
    })?;
    let mut value = Map::new();
    value.insert("name".to_owned(), json!(tool.name().as_str()));
    if !tool.description().is_empty() {
        value.insert("description".to_owned(), json!(tool.description()));
    }
    value.insert("input_schema".to_owned(), Value::Object(schema));
    Ok(Value::Object(value))
}

pub(crate) fn structured_format(output: &OutputMode) -> Result<Option<Value>, ClientError> {
    let OutputMode::Structured(shape) = output else {
        return Ok(None);
    };
    let mut schema = object_schema(shape.parameters(), Closure::Closed)
        .map_err(|reason| ClientError::OutputShapeRejected { reason })?;
    if let Some(description) = shape.description().filter(|text| !text.is_empty()) {
        schema.insert("description".to_owned(), json!(description));
    }
    Ok(Some(
        json!({"type": "json_schema", "schema": Value::Object(schema)}),
    ))
}
