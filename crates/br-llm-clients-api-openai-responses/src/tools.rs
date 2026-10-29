use br_llm_clients_core::{ClientError, ToolSpec};
use serde_json::{Map, Value};

use crate::schema::{is_strict, object_schema};

fn tool(spec: &ToolSpec) -> Result<Value, ClientError> {
    let parameters =
        object_schema(spec.parameters()).map_err(|_| ClientError::ToolSchemaRejected {
            tool: Some(spec.name().clone()),
            reason: "a parameter has a kind that this dialect does not know".to_owned(),
        })?;
    let mut fields = Map::new();
    fields.insert("type".to_owned(), Value::String("function".to_owned()));
    fields.insert(
        "name".to_owned(),
        Value::String(spec.name().as_str().to_owned()),
    );
    if !spec.description().is_empty() {
        fields.insert(
            "description".to_owned(),
            Value::String(spec.description().to_owned()),
        );
    }
    fields.insert("parameters".to_owned(), parameters);
    fields.insert(
        "strict".to_owned(),
        Value::Bool(is_strict(spec.parameters())),
    );
    Ok(Value::Object(fields))
}

pub(crate) fn tools(specs: &[ToolSpec]) -> Result<Vec<Value>, ClientError> {
    specs.iter().map(tool).collect()
}
