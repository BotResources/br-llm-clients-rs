use br_llm_clients_common_http::Secret;

use crate::build_error::MlxVlmBuildError;

fn invalid(reason: &str) -> MlxVlmBuildError {
    MlxVlmBuildError::InvalidCredential {
        reason: reason.to_owned(),
    }
}

pub(crate) fn check_key(key: &Secret) -> Result<(), MlxVlmBuildError> {
    let bytes = key.expose().as_bytes();
    if bytes.iter().any(|byte| !(0x21..=0x7E).contains(byte)) {
        return Err(invalid(
            "the key holds a byte outside the printable range 0x21 to 0x7E",
        ));
    }
    if bytes.iter().any(|byte| matches!(byte, b'"' | b'\\')) {
        return Err(invalid("the key holds a double quote or a backslash"));
    }
    Ok(())
}
