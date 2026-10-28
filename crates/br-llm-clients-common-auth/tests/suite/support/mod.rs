pub mod server;
pub mod source;

use std::sync::Arc;

use br_llm_clients_common_auth::TokenCache;
use br_llm_clients_common_http::{AuthorizerError, Secret};
use std::time::Duration;

use tokio::task::{JoinError, JoinHandle};
use tokio::time::{sleep, timeout};

use source::Scripted;

pub fn cache_of(source: &Arc<Scripted>) -> TokenCache {
    TokenCache::new(source.clone())
}

pub fn spawn_call(cache: &TokenCache) -> JoinHandle<Result<Secret, AuthorizerError>> {
    let cache = cache.clone();
    tokio::spawn(async move { cache.token().await })
}

pub async fn settle() {
    sleep(Duration::from_millis(1)).await;
}

pub async fn bounded<T>(handle: JoinHandle<T>) -> Result<T, JoinError> {
    timeout(Duration::from_secs(120), handle)
        .await
        .expect("the task did not end within 120 seconds")
}

pub async fn text_of(handle: JoinHandle<Result<Secret, AuthorizerError>>) -> String {
    bounded(handle)
        .await
        .expect("task")
        .expect("token")
        .expose()
        .to_owned()
}
