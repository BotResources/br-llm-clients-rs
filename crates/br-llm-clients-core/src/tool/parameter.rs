use std::collections::HashSet;

use br_llm_messages::PlainText;

use super::{Parameters, check_kind_depth, is_blank};
use crate::build_error::BuildError;

const STANDALONE_KIND: &str = "<kind>";

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    deny_unknown_fields,
    try_from = "RawParameterKind"
)]
#[non_exhaustive]
pub enum ParameterKind {
    String,
    Number,
    Integer,
    Boolean,
    Enum { values: Vec<String> },
    Array { items: Box<ParameterKind> },
    Object { parameters: Parameters },
}

#[derive(serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum RawParameterKind {
    String {},
    Number {},
    Integer {},
    Boolean {},
    Enum { values: Vec<String> },
    Array { items: Box<ParameterKind> },
    Object { parameters: Parameters },
}

impl TryFrom<RawParameterKind> for ParameterKind {
    type Error = BuildError;

    fn try_from(raw: RawParameterKind) -> Result<Self, Self::Error> {
        let kind = match raw {
            RawParameterKind::String {} => ParameterKind::String,
            RawParameterKind::Number {} => ParameterKind::Number,
            RawParameterKind::Integer {} => ParameterKind::Integer,
            RawParameterKind::Boolean {} => ParameterKind::Boolean,
            RawParameterKind::Enum { values } => ParameterKind::Enum { values },
            RawParameterKind::Array { items } => ParameterKind::Array { items },
            RawParameterKind::Object { parameters } => ParameterKind::Object { parameters },
        };
        check_kind(STANDALONE_KIND, &kind)?;
        check_kind_depth(STANDALONE_KIND, &kind)?;
        Ok(kind)
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "RawParameter")]
pub struct Parameter {
    name: PlainText,
    description: String,
    required: bool,
    kind: ParameterKind,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawParameter {
    name: String,
    description: String,
    required: bool,
    kind: ParameterKind,
}

impl TryFrom<RawParameter> for Parameter {
    type Error = BuildError;

    fn try_from(raw: RawParameter) -> Result<Self, Self::Error> {
        let parameter = Parameter::new(raw.name, raw.description, raw.required, raw.kind)?;
        check_kind_depth(parameter.name(), parameter.kind())?;
        Ok(parameter)
    }
}

impl Parameter {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        required: bool,
        kind: ParameterKind,
    ) -> Result<Self, BuildError> {
        let name = PlainText::new(name).map_err(|_| BuildError::BlankParameterName)?;
        check_kind(name.as_str(), &kind)?;
        Ok(Self {
            name,
            description: description.into(),
            required,
            kind,
        })
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn is_required(&self) -> bool {
        self.required
    }

    pub fn kind(&self) -> &ParameterKind {
        &self.kind
    }
}

fn check_kind(parameter: &str, kind: &ParameterKind) -> Result<(), BuildError> {
    match kind {
        ParameterKind::Enum { values } => check_enum(parameter, values),
        ParameterKind::Array { items } => check_kind(parameter, items),
        ParameterKind::String
        | ParameterKind::Number
        | ParameterKind::Integer
        | ParameterKind::Boolean
        | ParameterKind::Object { .. } => Ok(()),
    }
}

fn check_enum(parameter: &str, values: &[String]) -> Result<(), BuildError> {
    if values.is_empty() {
        return Err(BuildError::EmptyEnum {
            parameter: parameter.to_owned(),
        });
    }
    let mut seen = HashSet::new();
    for value in values {
        if is_blank(value) {
            return Err(BuildError::BlankEnumValue {
                parameter: parameter.to_owned(),
            });
        }
        if !seen.insert(value.as_str()) {
            return Err(BuildError::DuplicateEnumValue {
                parameter: parameter.to_owned(),
                value: value.clone(),
            });
        }
    }
    Ok(())
}
