use std::sync::Arc;

use br_llm_clients_common_http::{ApiKeyAuthorizer, Authorizer, BearerAuthorizer, Secret};

use crate::build_error::LiteLlmBuildError;
use crate::key_placement::KeyPlacement;

const LITELLM_KEY_HEADER: &str = "x-litellm-api-key";
const REFUSED_NAMES: [&str; 4] = [
    "authorization",
    LITELLM_KEY_HEADER,
    "accept",
    "content-type",
];

fn invalid(reason: &str) -> LiteLlmBuildError {
    LiteLlmBuildError::InvalidKeyHeader {
        reason: reason.to_owned(),
    }
}

pub(crate) fn check_placement(
    placement: &KeyPlacement,
    key: &Secret,
) -> Result<(), LiteLlmBuildError> {
    let KeyPlacement::ConfiguredKeyHeader(name) = placement else {
        return Ok(());
    };
    if REFUSED_NAMES
        .iter()
        .any(|refused| refused.eq_ignore_ascii_case(name))
    {
        return Err(invalid(
            "the header name is one that the library refuses for a configured key header: authorization, x-litellm-api-key, accept or content-type",
        ));
    }
    ApiKeyAuthorizer::new(name, key.clone())
        .map(drop)
        .map_err(|_| invalid("the header name is not valid or is managed by the HTTP library"))
}

fn bearer_value(key: &Secret) -> Result<Secret, LiteLlmBuildError> {
    Secret::new(format!("Bearer {}", key.expose()))
        .map_err(|_| invalid("the key header value could not be made"))
}

pub(crate) fn authorizer(
    placement: &KeyPlacement,
    key: &Secret,
) -> Result<Arc<dyn Authorizer>, LiteLlmBuildError> {
    match placement {
        KeyPlacement::AuthorizationBearer => Ok(Arc::new(BearerAuthorizer::new(key))),
        KeyPlacement::LiteLlmKeyHeader => header_authorizer(LITELLM_KEY_HEADER, key),
        KeyPlacement::ConfiguredKeyHeader(name) => header_authorizer(name, key),
    }
}

fn header_authorizer(name: &str, key: &Secret) -> Result<Arc<dyn Authorizer>, LiteLlmBuildError> {
    let authorizer = ApiKeyAuthorizer::new(name, bearer_value(key)?)
        .map_err(|_| invalid("the header name is not valid or is managed by the HTTP library"))?;
    Ok(Arc::new(authorizer))
}
