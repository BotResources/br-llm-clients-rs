use br_llm_clients_common_call::Secret;

use crate::build_error::OpenAiBuildError;

const CREDENTIAL_REASON: &str = "the key holds a character outside the range from `!` to `~`";
const HEADER_VALUE_REASON: &str =
    "the value is empty or holds a character outside the range from `!` to `~`";

fn is_visible_ascii(text: &str) -> bool {
    text.bytes().all(|byte| (b'!'..=b'~').contains(&byte))
}

pub(crate) fn credential(key: &Secret) -> Result<(), OpenAiBuildError> {
    if is_visible_ascii(key.expose()) {
        return Ok(());
    }
    Err(OpenAiBuildError::InvalidCredential {
        reason: CREDENTIAL_REASON.to_owned(),
    })
}

pub(crate) fn header_value(
    value: Option<&str>,
    refused: fn(String) -> OpenAiBuildError,
) -> Result<(), OpenAiBuildError> {
    match value {
        Some(value) if value.is_empty() || !is_visible_ascii(value) => {
            Err(refused(HEADER_VALUE_REASON.to_owned()))
        }
        Some(_) | None => Ok(()),
    }
}
