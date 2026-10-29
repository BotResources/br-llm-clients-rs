use br_llm_clients_common_http::{ConfigError, Endpoint, Request};

use crate::build_error::LiteLlmBuildError;
use crate::key_header::{ACCEPT_HEADER, AUTHORIZATION_HEADER, LITELLM_KEY_HEADER};

const REFUSED_NAMES: [&str; 3] = [ACCEPT_HEADER, AUTHORIZATION_HEADER, LITELLM_KEY_HEADER];
const PROBE_NAME: &str = "x-litellm-probe";
const PROBE_VALUE: &str = "probe";
const REFUSED_NAME: &str =
    "has a name that the library refuses: accept, authorization or x-litellm-api-key";
const INVALID_VALUE: &str = "has a value that is not a valid header value";

fn invalid(index: usize, fault: &str) -> LiteLlmBuildError {
    LiteLlmBuildError::InvalidExtraHeader {
        reason: format!("the extra header at index {index} (counted from 0) {fault}"),
    }
}

fn name_fault(error: &ConfigError) -> &'static str {
    match error {
        ConfigError::DuplicateHeader { .. } => "has a name that is given twice",
        ConfigError::ReservedHeader { .. } => "has a name that the HTTP library manages",
        _ => "has a name that is not a valid header name",
    }
}

pub(crate) fn check_extra_headers(
    headers: &[(String, String)],
    base: &Endpoint,
) -> Result<(), LiteLlmBuildError> {
    let mut names = Request::new(base.clone(), Vec::new());
    for (index, (name, value)) in headers.iter().enumerate() {
        if REFUSED_NAMES
            .iter()
            .any(|refused| refused.eq_ignore_ascii_case(name))
        {
            return Err(invalid(index, REFUSED_NAME));
        }
        names = names
            .with_header(name, PROBE_VALUE)
            .map_err(|error| invalid(index, name_fault(&error)))?;
        Request::new(base.clone(), Vec::new())
            .with_header(PROBE_NAME, value)
            .map_err(|_| invalid(index, INVALID_VALUE))?;
    }
    Ok(())
}
