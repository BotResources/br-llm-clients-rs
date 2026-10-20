use br_llm_clients_common_call::Secret;

use crate::build_error::OpenAiBuildError;

const HEADER_VALUE_REASON: &str =
    "the value is empty or holds a character outside the range from `!` to `~`";

fn is_visible_ascii(text: &str) -> bool {
    text.bytes().all(|byte| (b'!'..=b'~').contains(&byte))
}

fn is_header_text(value: &str) -> bool {
    !value.is_empty() && is_visible_ascii(value)
}

pub(crate) fn credential(key: &Secret) -> Result<(), OpenAiBuildError> {
    if is_visible_ascii(key.expose()) {
        return Ok(());
    }
    Err(OpenAiBuildError::InvalidCredential {
        reason: "the key holds a character outside the range from `!` to `~`".to_owned(),
    })
}

pub(crate) fn organization(value: &str) -> Result<(), OpenAiBuildError> {
    if is_header_text(value) {
        return Ok(());
    }
    Err(OpenAiBuildError::InvalidOrganization {
        reason: HEADER_VALUE_REASON.to_owned(),
    })
}

pub(crate) fn project(value: &str) -> Result<(), OpenAiBuildError> {
    if is_header_text(value) {
        return Ok(());
    }
    Err(OpenAiBuildError::InvalidProject {
        reason: HEADER_VALUE_REASON.to_owned(),
    })
}
