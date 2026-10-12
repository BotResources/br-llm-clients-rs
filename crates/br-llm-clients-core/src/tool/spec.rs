use br_llm_messages::ToolName;

use super::{Parameters, check_depth};
use crate::build_error::BuildError;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "RawToolSpec")]
pub struct ToolSpec {
    name: ToolName,
    description: String,
    parameters: Parameters,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawToolSpec {
    name: ToolName,
    description: String,
    parameters: Parameters,
}

impl TryFrom<RawToolSpec> for ToolSpec {
    type Error = BuildError;

    fn try_from(raw: RawToolSpec) -> Result<Self, Self::Error> {
        ToolSpec::new(raw.name, raw.description, raw.parameters)
    }
}

impl ToolSpec {
    pub fn new(
        name: ToolName,
        description: impl Into<String>,
        parameters: Parameters,
    ) -> Result<Self, BuildError> {
        check_depth(&parameters)?;
        Ok(Self {
            name,
            description: description.into(),
            parameters,
        })
    }

    pub fn name(&self) -> &ToolName {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn parameters(&self) -> &Parameters {
        &self.parameters
    }
}
