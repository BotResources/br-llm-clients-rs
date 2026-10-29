use serde_json::{Map, Value};

use crate::build_error::VertexBuildError;

pub(crate) struct KeyFields {
    members: Map<String, Value>,
}

impl KeyFields {
    pub(crate) fn parse(text: &str) -> Result<Self, VertexBuildError> {
        match serde_json::from_str::<Value>(text) {
            Ok(Value::Object(members)) => Ok(Self { members }),
            _ => Err(VertexBuildError::credential(
                "the service account key is not a JSON object",
            )),
        }
    }

    pub(crate) fn required(&self, name: &str) -> Result<&str, VertexBuildError> {
        match self.members.get(name) {
            Some(Value::String(text)) if !text.is_empty() => Ok(text),
            _ => Err(VertexBuildError::credential(format!(
                "the member `{name}` of the service account key is missing, empty or not a text"
            ))),
        }
    }

    pub(crate) fn optional(&self, name: &str) -> Result<Option<&str>, VertexBuildError> {
        match self.members.get(name) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(text)) => Ok(Some(text)),
            Some(_) => Err(VertexBuildError::credential(format!(
                "the member `{name}` of the service account key is not a text"
            ))),
        }
    }
}
