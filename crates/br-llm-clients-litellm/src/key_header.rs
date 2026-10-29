use std::sync::Arc;

use br_llm_clients_common_http::{
    ApiKeyAuthorizer, Authorizer, BearerAuthorizer, ConfigError, Secret,
};

use crate::build_error::LiteLlmBuildError;
use crate::key_placement::KeyPlacement;

pub(crate) const AUTHORIZATION_HEADER: &str = "authorization";
pub(crate) const ACCEPT_HEADER: &str = "accept";
pub(crate) const LITELLM_KEY_HEADER: &str = "x-litellm-api-key";
const CONTENT_TYPE_HEADER: &str = "content-type";
const REFUSED_NAMES: [&str; 4] = [
    AUTHORIZATION_HEADER,
    LITELLM_KEY_HEADER,
    ACCEPT_HEADER,
    CONTENT_TYPE_HEADER,
];
const REFUSED_NAME_REASON: &str = "the header name is one that the library refuses for a configured key header: authorization, x-litellm-api-key, accept or content-type";
const INVALID_NAME_REASON: &str = "the header name is not a valid header name";
const RESERVED_NAME_REASON: &str = "the header name is managed by the HTTP library";
const INVALID_VALUE_REASON: &str = "the key header value could not be made";

fn invalid(reason: &str) -> LiteLlmBuildError {
    LiteLlmBuildError::InvalidKeyHeader {
        reason: reason.to_owned(),
    }
}

pub(crate) fn key_authorizer(
    placement: &KeyPlacement,
    key: &Secret,
) -> Result<Arc<dyn Authorizer>, LiteLlmBuildError> {
    match placement {
        KeyPlacement::AuthorizationBearer => Ok(Arc::new(BearerAuthorizer::new(key))),
        KeyPlacement::LiteLlmKeyHeader => header_authorizer(LITELLM_KEY_HEADER, key),
        KeyPlacement::ConfiguredKeyHeader(name) => {
            if REFUSED_NAMES
                .iter()
                .any(|refused| refused.eq_ignore_ascii_case(name))
            {
                return Err(invalid(REFUSED_NAME_REASON));
            }
            header_authorizer(name, key)
        }
    }
}

fn header_authorizer(name: &str, key: &Secret) -> Result<Arc<dyn Authorizer>, LiteLlmBuildError> {
    let value = Secret::new(format!("Bearer {}", key.expose()))
        .map_err(|_| invalid(INVALID_VALUE_REASON))?;
    let authorizer = ApiKeyAuthorizer::new(name, value).map_err(|error| match error {
        ConfigError::ReservedHeader { .. } => invalid(RESERVED_NAME_REASON),
        _ => invalid(INVALID_NAME_REASON),
    })?;
    Ok(Arc::new(authorizer))
}
