use br_llm_messages::PlainText;

use crate::build_error::BuildError;
use crate::tool::{Parameters, check_depth};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", from = "RawOutputMode")]
pub enum OutputMode {
    Plain,
    Structured(StructuredShape),
}

#[derive(serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum RawOutputMode {
    Plain {},
    Structured(StructuredShape),
}

impl From<RawOutputMode> for OutputMode {
    fn from(raw: RawOutputMode) -> Self {
        match raw {
            RawOutputMode::Plain {} => OutputMode::Plain,
            RawOutputMode::Structured(shape) => OutputMode::Structured(shape),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "RawStructuredShape")]
pub struct StructuredShape {
    name: PlainText,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    parameters: Parameters,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStructuredShape {
    name: String,
    #[serde(default)]
    description: Option<String>,
    parameters: Parameters,
}

impl TryFrom<RawStructuredShape> for StructuredShape {
    type Error = BuildError;

    fn try_from(raw: RawStructuredShape) -> Result<Self, Self::Error> {
        StructuredShape::new(raw.name, raw.description, raw.parameters)
    }
}

impl StructuredShape {
    pub fn new(
        name: impl Into<String>,
        description: Option<String>,
        parameters: Parameters,
    ) -> Result<Self, BuildError> {
        let name = PlainText::new(name).map_err(|_| BuildError::BlankShapeName)?;
        check_depth(&parameters)?;
        Ok(Self {
            name,
            description,
            parameters,
        })
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn parameters(&self) -> &Parameters {
        &self.parameters
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{parameters, scalar};
    use crate::{Parameter, ParameterKind};

    #[test]
    fn given_a_blank_shape_name_when_built_then_refused() {
        for name in ["", "  "] {
            assert_eq!(
                StructuredShape::new(name, None, Parameters::default()),
                Err(BuildError::BlankShapeName)
            );
        }
    }

    #[test]
    fn given_a_shape_without_parameters_when_built_then_accepted() {
        assert!(StructuredShape::new("empty", None, Parameters::default()).is_ok());
    }

    #[test]
    fn given_a_shape_nested_too_deep_when_built_then_refused_like_a_tool() {
        let mut kind = ParameterKind::String;
        for _ in 0..33 {
            kind = ParameterKind::Array {
                items: Box::new(kind),
            };
        }
        let list = parameters(vec![Parameter::new("a", "", true, kind).unwrap()]);
        assert_eq!(
            StructuredShape::new("shape", None, list),
            Err(BuildError::NestingTooDeep {
                parameter: "a".to_owned()
            })
        );
    }

    #[test]
    fn given_each_mode_when_round_tripped_then_identical() {
        let shape = StructuredShape::new(
            "answer",
            Some("the final answer".to_owned()),
            parameters(vec![scalar("value")]),
        )
        .unwrap();
        for mode in [OutputMode::Plain, OutputMode::Structured(shape)] {
            let json = serde_json::to_value(&mode).unwrap();
            assert_eq!(serde_json::from_value::<OutputMode>(json).unwrap(), mode);
        }
    }

    #[test]
    fn given_a_stored_shape_with_a_blank_name_when_loaded_then_refused() {
        let stored = serde_json::json!({
            "type": "structured", "name": " ", "parameters": []
        });
        assert!(serde_json::from_value::<OutputMode>(stored).is_err());
    }

    #[test]
    fn given_a_stored_shape_nested_too_deep_when_loaded_then_refused() {
        let nested_to = |layers: usize| {
            let mut kind = serde_json::json!({ "type": "string" });
            for _ in 0..layers {
                kind = serde_json::json!({ "type": "array", "items": kind });
            }
            serde_json::json!({
                "type": "structured", "name": "shape",
                "parameters": [
                    { "name": "a", "description": "", "required": true, "kind": kind }
                ]
            })
        };
        assert!(serde_json::from_value::<OutputMode>(nested_to(32)).is_ok());
        assert!(serde_json::from_value::<OutputMode>(nested_to(33)).is_err());
    }

    #[test]
    fn given_a_mode_unknown_to_this_version_when_loaded_then_refused() {
        let stored = serde_json::json!({ "type": "json_schema" });
        assert!(serde_json::from_value::<OutputMode>(stored).is_err());
    }
}
