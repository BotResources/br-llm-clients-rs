use crate::build_error::VertexBuildError;

const MAX_LOCATION_BYTES: usize = 32;

pub(crate) fn check(location: &str) -> Result<(), VertexBuildError> {
    if is_location(location) {
        Ok(())
    } else {
        Err(VertexBuildError::location(
            "a location is 1 to 32 bytes of lowercase ASCII letters, digits and hyphens, starts with a letter, does not end with a hyphen and holds no two hyphens in a row",
        ))
    }
}

fn is_location(location: &str) -> bool {
    let bytes = location.as_bytes();
    let starts_with_letter = bytes.first().is_some_and(u8::is_ascii_lowercase);
    let allowed = bytes
        .iter()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-');
    starts_with_letter
        && bytes.len() <= MAX_LOCATION_BYTES
        && allowed
        && !location.ends_with('-')
        && !location.contains("--")
}
