use br_llm_clients_common_http::Secret;

use crate::build_error::KimiBuildError;

pub(crate) fn check_key(key: &Secret) -> Result<(), KimiBuildError> {
    if key.expose().chars().all(|c| ('!'..='~').contains(&c)) {
        return Ok(());
    }
    Err(KimiBuildError::InvalidCredential {
        reason: "the key holds a character outside the range from `!` to `~`".to_owned(),
    })
}
