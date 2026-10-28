use br_llm_messages::OpaqueJson;

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum JsonInput {
    Value(OpaqueJson),
    Text(String),
}

impl JsonInput {
    pub(crate) fn parsed(&self) -> Result<OpaqueJson, br_llm_messages::Error> {
        match self {
            JsonInput::Value(value) => Ok(value.clone()),
            JsonInput::Text(text) => OpaqueJson::parse(text),
        }
    }
}

impl From<OpaqueJson> for JsonInput {
    fn from(value: OpaqueJson) -> Self {
        JsonInput::Value(value)
    }
}

impl From<&str> for JsonInput {
    fn from(text: &str) -> Self {
        JsonInput::Text(text.to_owned())
    }
}

impl From<String> for JsonInput {
    fn from(text: String) -> Self {
        JsonInput::Text(text)
    }
}
