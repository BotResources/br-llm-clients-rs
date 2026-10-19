use br_llm_clients_core::{ClientError, ToolSpec};
use serde_json::{Map, Value};

use crate::encode_schema::{UnknownKind, object_schema};

pub(crate) fn tools(specs: &[ToolSpec]) -> Result<Option<Value>, ClientError> {
    if specs.is_empty() {
        return Ok(None);
    }
    let encoded = specs.iter().map(tool).collect::<Result<Vec<_>, _>>()?;
    Ok(Some(Value::Array(encoded)))
}

fn tool(spec: &ToolSpec) -> Result<Value, ClientError> {
    let parameters =
        object_schema(spec.parameters()).map_err(|unknown| rejection(spec, unknown))?;
    let mut function = Map::new();
    function.insert("name".to_owned(), Value::from(spec.name().as_str()));
    if !spec.description().is_empty() {
        function.insert("description".to_owned(), Value::from(spec.description()));
    }
    function.insert("parameters".to_owned(), parameters);
    let mut tool = Map::new();
    tool.insert("type".to_owned(), Value::from("function"));
    tool.insert("function".to_owned(), Value::Object(function));
    Ok(Value::Object(tool))
}

fn rejection(spec: &ToolSpec, unknown: UnknownKind) -> ClientError {
    ClientError::ToolSchemaRejected {
        tool: Some(spec.name().clone()),
        reason: format!(
            "the parameter `{}` has a kind that this dialect does not know",
            unknown.parameter
        ),
    }
}
