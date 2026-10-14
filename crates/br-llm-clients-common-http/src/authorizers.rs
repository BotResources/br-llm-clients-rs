use http::HeaderName;

use crate::authorizer::{AuthorizationRequest, AuthorizedHeader, Authorizer, BoxFuture};
use crate::authorizer_error::AuthorizerError;
use crate::config_error::ConfigError;
use crate::header::{describe_name, is_reserved};
use crate::secret::Secret;

#[derive(Clone)]
pub struct ApiKeyAuthorizer {
    name: String,
    key: Secret,
}

impl ApiKeyAuthorizer {
    pub fn new(header_name: &str, key: Secret) -> Result<Self, ConfigError> {
        let shown = describe_name(header_name);
        let parsed = HeaderName::from_bytes(header_name.as_bytes()).map_err(|_| {
            ConfigError::InvalidHeader {
                text: format!("`{shown}` is not a valid header name"),
            }
        })?;
        if is_reserved(&parsed) {
            return Err(ConfigError::ReservedHeader {
                text: format!("`{shown}` is managed by the library"),
            });
        }
        Ok(Self {
            name: parsed.as_str().to_owned(),
            key,
        })
    }
}

impl Authorizer for ApiKeyAuthorizer {
    fn authorize<'a>(
        &'a self,
        _request: &'a AuthorizationRequest<'a>,
    ) -> BoxFuture<'a, Result<Vec<AuthorizedHeader>, AuthorizerError>> {
        Box::pin(async move {
            Ok(vec![AuthorizedHeader::new(
                self.name.clone(),
                self.key.clone(),
            )])
        })
    }
}

#[derive(Clone)]
pub struct BearerAuthorizer {
    value: Secret,
}

impl BearerAuthorizer {
    pub fn new(token: &Secret) -> Self {
        Self {
            value: Secret::from_non_empty(format!("Bearer {}", token.expose())),
        }
    }
}

impl Authorizer for BearerAuthorizer {
    fn authorize<'a>(
        &'a self,
        _request: &'a AuthorizationRequest<'a>,
    ) -> BoxFuture<'a, Result<Vec<AuthorizedHeader>, AuthorizerError>> {
        Box::pin(async move {
            Ok(vec![AuthorizedHeader::new(
                "authorization",
                self.value.clone(),
            )])
        })
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NoAuthorizer;

impl Authorizer for NoAuthorizer {
    fn authorize<'a>(
        &'a self,
        _request: &'a AuthorizationRequest<'a>,
    ) -> BoxFuture<'a, Result<Vec<AuthorizedHeader>, AuthorizerError>> {
        Box::pin(async move { Ok(Vec::new()) })
    }
}

impl std::fmt::Debug for ApiKeyAuthorizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiKeyAuthorizer")
            .field("header", &self.name)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for BearerAuthorizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BearerAuthorizer").finish_non_exhaustive()
    }
}
