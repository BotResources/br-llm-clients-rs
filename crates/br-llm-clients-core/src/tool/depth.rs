use super::{ParameterKind, Parameters};
use crate::build_error::BuildError;

pub const MAX_NESTING_DEPTH: usize = 32;

pub(crate) fn check_depth(parameters: &Parameters) -> Result<(), BuildError> {
    for parameter in parameters {
        check_kind(parameter.name(), parameter.kind(), 0)?;
    }
    Ok(())
}

pub(crate) fn check_kind_depth(owner: &str, kind: &ParameterKind) -> Result<(), BuildError> {
    check_kind(owner, kind, 0)
}

fn check_kind(owner: &str, kind: &ParameterKind, depth: usize) -> Result<(), BuildError> {
    match kind {
        ParameterKind::Array { items } => {
            let item_depth = depth + 1;
            if item_depth > MAX_NESTING_DEPTH {
                return Err(too_deep(owner));
            }
            check_kind(owner, items, item_depth)
        }
        ParameterKind::Object { parameters } => {
            let inner_depth = depth + 1;
            for inner in parameters {
                if inner_depth > MAX_NESTING_DEPTH {
                    return Err(too_deep(inner.name()));
                }
                check_kind(inner.name(), inner.kind(), inner_depth)?;
            }
            Ok(())
        }
        ParameterKind::String
        | ParameterKind::Number
        | ParameterKind::Integer
        | ParameterKind::Boolean
        | ParameterKind::Enum { .. } => Ok(()),
    }
}

fn too_deep(parameter: &str) -> BuildError {
    BuildError::NestingTooDeep {
        parameter: parameter.to_owned(),
    }
}
