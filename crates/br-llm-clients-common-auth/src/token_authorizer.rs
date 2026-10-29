use std::fmt;
use std::sync::Arc;

use br_llm_clients_common_http::{
    AuthorizationRequest, AuthorizedHeader, Authorizer, AuthorizerError, BearerAuthorizer,
    BoxFuture, Secret,
};

use crate::token_cache::TokenCache;

type Observer = Arc<dyn Fn(&Secret) + Send + Sync>;

#[derive(Clone)]
pub struct TokenAuthorizer {
    cache: TokenCache,
    observer: Option<Observer>,
}

impl TokenAuthorizer {
    pub fn bearer(cache: TokenCache) -> Self {
        Self {
            cache,
            observer: None,
        }
    }

    pub fn with_token_observer(mut self, observer: Arc<dyn Fn(&Secret) + Send + Sync>) -> Self {
        self.observer = Some(observer);
        self
    }
}

impl Authorizer for TokenAuthorizer {
    fn authorize<'a>(
        &'a self,
        request: &'a AuthorizationRequest<'a>,
    ) -> BoxFuture<'a, Result<Vec<AuthorizedHeader>, AuthorizerError>> {
        Box::pin(async move {
            let token = self.cache.token().await?;
            if let Some(observer) = &self.observer {
                observer(&token);
            }
            BearerAuthorizer::new(&token).authorize(request).await
        })
    }
}

impl fmt::Debug for TokenAuthorizer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TokenAuthorizer")
            .field("observer", &self.observer.is_some())
            .finish_non_exhaustive()
    }
}
