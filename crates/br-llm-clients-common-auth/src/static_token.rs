use std::fmt;

use br_llm_clients_common_http::{AuthorizerError, BoxFuture, Secret};

use crate::access_token::AccessToken;
use crate::token_source::TokenSource;

pub struct StaticToken {
    token: Secret,
}

impl StaticToken {
    pub fn new(token: Secret) -> Self {
        Self { token }
    }
}

impl TokenSource for StaticToken {
    fn fetch(&self) -> BoxFuture<'_, Result<AccessToken, AuthorizerError>> {
        Box::pin(async move { Ok(AccessToken::new(self.token.clone(), None)) })
    }

    fn secrets(&self) -> Vec<Secret> {
        vec![self.token.clone()]
    }
}

impl fmt::Debug for StaticToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StaticToken").finish_non_exhaustive()
    }
}
