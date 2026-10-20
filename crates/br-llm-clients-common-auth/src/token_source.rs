use br_llm_clients_common_http::{AuthorizerError, BoxFuture, Secret};

use crate::access_token::AccessToken;

pub trait TokenSource: Send + Sync {
    fn fetch(&self) -> BoxFuture<'_, Result<AccessToken, AuthorizerError>>;

    fn secrets(&self) -> Vec<Secret> {
        Vec::new()
    }
}
