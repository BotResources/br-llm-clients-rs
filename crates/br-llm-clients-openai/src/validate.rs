use br_llm_clients_common_call::Secret;

use crate::build_error::OpenAiBuildError;

fn is_visible_ascii(text: &str) -> bool {
    text.bytes().all(|byte| (b'!'..=b'~').contains(&byte))
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
    if !value.is_empty() && is_visible_ascii(value) {
        return Ok(());
    }
    Err(OpenAiBuildError::InvalidOrganization {
        reason: "the value is empty or holds a character outside the range from `!` to `~`"
            .to_owned(),
    })
}

pub(crate) fn project(value: &str) -> Result<(), OpenAiBuildError> {
    if !value.is_empty() && is_visible_ascii(value) {
        return Ok(());
    }
    Err(OpenAiBuildError::InvalidProject {
        reason: "the value is empty or holds a character outside the range from `!` to `~`"
            .to_owned(),
    })
}
