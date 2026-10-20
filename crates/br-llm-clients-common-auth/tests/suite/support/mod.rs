pub mod server;
pub mod source;

use std::sync::Arc;

use br_llm_clients_common_auth::TokenCache;
use br_llm_clients_common_http::{AuthorizerError, Secret};
use tokio::task::{JoinHandle, yield_now};

use source::Scripted;

pub fn cache_of(source: &Arc<Scripted>) -> TokenCache {
    TokenCache::new(source.clone())
}

pub fn spawn_call(cache: &TokenCache) -> JoinHandle<Result<Secret, AuthorizerError>> {
    let cache = cache.clone();
    tokio::spawn(async move { cache.token().await })
}

pub async fn settle() {
    for _ in 0..16 {
        yield_now().await;
    }
}

pub async fn text_of(handle: JoinHandle<Result<Secret, AuthorizerError>>) -> String {
    handle
        .await
        .expect("task")
        .expect("token")
        .expose()
        .to_owned()
}
