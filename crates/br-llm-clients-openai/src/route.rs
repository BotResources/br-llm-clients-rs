use br_llm_clients_common_call::Endpoint;

use crate::build_error::OpenAiBuildError;

const DEFAULT_BASE: &str = "https://api.openai.com";

fn invalid(error: impl ToString) -> OpenAiBuildError {
    OpenAiBuildError::InvalidEndpoint {
        reason: error.to_string(),
    }
}

pub(crate) fn responses_endpoint(base: Option<Endpoint>) -> Result<Endpoint, OpenAiBuildError> {
    let base = match base {
        Some(base) => base,
        None => Endpoint::https(DEFAULT_BASE).map_err(invalid)?,
    };
    base.join_segment("v1")
        .and_then(|versioned| versioned.join_segment("responses"))
        .map_err(invalid)
}
