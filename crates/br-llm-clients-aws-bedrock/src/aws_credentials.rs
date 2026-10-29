use std::fmt;

use br_llm_clients_common_call::Secret;
use br_llm_clients_common_http::{AuthorizerError, BoxFuture};

use crate::build_error::BedrockBuildError;
use crate::visible::is_visible;

#[derive(Clone)]
pub struct AwsCredentials {
    access_key_id: String,
    secret_access_key: Secret,
    session_token: Option<Secret>,
}

impl AwsCredentials {
    pub fn new(
        access_key_id: impl Into<String>,
        secret_access_key: Secret,
    ) -> Result<Self, BedrockBuildError> {
        let access_key_id = access_key_id.into();
        if !is_visible(&access_key_id) {
            return Err(BedrockBuildError::credential(
                "the access key id is empty or is not visible ASCII text",
            ));
        }
        Ok(Self {
            access_key_id,
            secret_access_key,
            session_token: None,
        })
    }

    pub fn with_session_token(mut self, session_token: Secret) -> Self {
        self.session_token = Some(session_token);
        self
    }

    pub(crate) fn access_key_id(&self) -> &str {
        &self.access_key_id
    }

    pub(crate) fn secret_access_key(&self) -> &Secret {
        &self.secret_access_key
    }

    pub(crate) fn session_token(&self) -> Option<&Secret> {
        self.session_token.as_ref()
    }
}

impl fmt::Debug for AwsCredentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AwsCredentials")
            .field("session_token_set", &self.session_token.is_some())
            .finish_non_exhaustive()
    }
}

pub trait CredentialSource: Send + Sync {
    fn credentials(&self) -> BoxFuture<'_, Result<AwsCredentials, AuthorizerError>>;
}

#[derive(Clone)]
pub struct StaticCredentials {
    credentials: AwsCredentials,
}

impl StaticCredentials {
    pub fn new(credentials: AwsCredentials) -> Self {
        Self { credentials }
    }
}

impl CredentialSource for StaticCredentials {
    fn credentials(&self) -> BoxFuture<'_, Result<AwsCredentials, AuthorizerError>> {
        Box::pin(async move { Ok(self.credentials.clone()) })
    }
}

impl fmt::Debug for StaticCredentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StaticCredentials")
            .field("credentials", &self.credentials)
            .finish()
    }
}
