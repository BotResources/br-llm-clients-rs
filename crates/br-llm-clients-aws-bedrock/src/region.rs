use crate::build_error::BedrockBuildError;

const MAX_REGION_BYTES: usize = 32;

pub(crate) fn check(region: &str) -> Result<(), BedrockBuildError> {
    if is_region(region) {
        Ok(())
    } else {
        Err(BedrockBuildError::region(
            "a region is 1 to 32 bytes of lowercase ASCII letters, digits and hyphens, starts with a letter, does not end with a hyphen and holds no two hyphens in a row",
        ))
    }
}

fn is_region(region: &str) -> bool {
    let bytes = region.as_bytes();
    let starts_with_letter = bytes.first().is_some_and(u8::is_ascii_lowercase);
    let allowed = bytes
        .iter()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-');
    starts_with_letter
        && bytes.len() <= MAX_REGION_BYTES
        && allowed
        && !region.ends_with('-')
        && !region.contains("--")
}
