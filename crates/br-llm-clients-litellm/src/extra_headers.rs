use br_llm_clients_common_http::{ConfigError, Endpoint, Request};

use crate::build_error::LiteLlmBuildError;

const REFUSED_NAMES: [&str; 3] = ["accept", "authorization", "x-litellm-api-key"];

fn invalid(position: usize, fault: &str) -> LiteLlmBuildError {
    LiteLlmBuildError::InvalidExtraHeader {
        reason: format!("the extra header at position {position} {fault}"),
    }
}

pub(crate) fn check_extra_headers(
    headers: &[(String, String)],
    base: &Endpoint,
) -> Result<(), LiteLlmBuildError> {
    let mut scratch = Request::new(base.clone(), Vec::new());
    for (position, (name, value)) in headers.iter().enumerate() {
        if REFUSED_NAMES
            .iter()
            .any(|refused| refused.eq_ignore_ascii_case(name))
        {
            return Err(invalid(
                position,
                "has a name that the library refuses: accept, authorization or x-litellm-api-key",
            ));
        }
        scratch = scratch.with_header(name, value).map_err(|error| {
            let fault = match error {
                ConfigError::DuplicateHeader { .. } => "has a name that is given twice",
                ConfigError::ReservedHeader { .. } => "has a name that the HTTP library manages",
                _ => "has a name or a value that the HTTP library refuses",
            };
            invalid(position, fault)
        })?;
    }
    Ok(())
}
