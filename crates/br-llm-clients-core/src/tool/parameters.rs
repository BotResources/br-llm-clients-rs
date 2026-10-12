use std::collections::HashSet;

use super::{Parameter, check_depth};
use crate::build_error::BuildError;

#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "RawParameters", into = "Vec<Parameter>")]
pub struct Parameters(Vec<Parameter>);

impl Parameters {
    pub fn new(parameters: Vec<Parameter>) -> Result<Self, BuildError> {
        let mut seen = HashSet::new();
        for parameter in &parameters {
            if !seen.insert(parameter.name()) {
                return Err(BuildError::DuplicateParameterName {
                    parameter: parameter.name().to_owned(),
                });
            }
        }
        Ok(Self(parameters))
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Parameter> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(serde::Deserialize)]
struct RawParameters(Vec<Parameter>);

impl TryFrom<RawParameters> for Parameters {
    type Error = BuildError;

    fn try_from(raw: RawParameters) -> Result<Self, Self::Error> {
        let parameters = Self::new(raw.0)?;
        check_depth(&parameters)?;
        Ok(parameters)
    }
}

impl From<Parameters> for Vec<Parameter> {
    fn from(parameters: Parameters) -> Self {
        parameters.0
    }
}

impl<'a> IntoIterator for &'a Parameters {
    type Item = &'a Parameter;
    type IntoIter = std::slice::Iter<'a, Parameter>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
