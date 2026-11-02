use std::ops::RangeInclusive;

use crate::build_error::VertexBuildError;

const PROJECT_ID_BYTES: RangeInclusive<usize> = 6..=30;
const PROJECT_NUMBER_BYTES: RangeInclusive<usize> = 1..=30;

pub(crate) fn check(project: &str) -> Result<(), VertexBuildError> {
    if is_project_id(project) || is_project_number(project) {
        Ok(())
    } else {
        Err(VertexBuildError::project(
            "a project is a project id (6 to 30 bytes of lowercase ASCII letters, digits and hyphens, starting with a letter and not ending with a hyphen) or a project number (1 to 30 ASCII digits)",
        ))
    }
}

fn is_project_id(project: &str) -> bool {
    let bytes = project.as_bytes();
    PROJECT_ID_BYTES.contains(&bytes.len())
        && bytes.first().is_some_and(u8::is_ascii_lowercase)
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
        && !project.ends_with('-')
}

fn is_project_number(project: &str) -> bool {
    PROJECT_NUMBER_BYTES.contains(&project.len())
        && project.bytes().all(|byte| byte.is_ascii_digit())
}
