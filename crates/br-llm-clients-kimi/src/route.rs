use br_llm_clients_common_http::Endpoint;

use crate::build_error::KimiBuildError;

pub(crate) const DEFAULT_BASE: &str = "https://api.moonshot.ai";

pub(crate) fn chat_route(base: Option<&Endpoint>) -> Result<Endpoint, KimiBuildError> {
    let base = match base {
        Some(base) => base.clone(),
        None => Endpoint::https(DEFAULT_BASE).map_err(invalid)?,
    };
    base.join_segment("v1")
        .and_then(|endpoint| endpoint.join_segment("chat"))
        .and_then(|endpoint| endpoint.join_segment("completions"))
        .map_err(invalid)
}

fn invalid(error: br_llm_clients_common_http::ConfigError) -> KimiBuildError {
    KimiBuildError::InvalidEndpoint {
        reason: error.to_string(),
    }
}
