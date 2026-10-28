pub mod server;
pub mod source;

use std::future::Future;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use br_llm_clients_common_auth::TokenCache;
use br_llm_clients_common_http::{AuthorizerError, Secret};

use source::Scripted;

pub type Call<'a> = Pin<Box<dyn Future<Output = Result<Secret, AuthorizerError>> + Send + 'a>>;

pub fn cache_of(source: &Arc<Scripted>) -> TokenCache {
    TokenCache::new(source.clone())
}

pub fn poll_once<F: Future + Unpin>(future: &mut F) -> Poll<F::Output> {
    Pin::new(future).poll(&mut Context::from_waker(Waker::noop()))
}

pub fn call(cache: &TokenCache) -> Call<'_> {
    Box::pin(cache.token())
}

pub fn parked(cache: &TokenCache) -> Call<'_> {
    let mut pending = call(cache);
    assert!(
        poll_once(&mut pending).is_pending(),
        "the call ended at once instead of waiting"
    );
    pending
}

pub fn at_once(cache: &TokenCache) -> Result<Secret, AuthorizerError> {
    match poll_once(&mut call(cache)) {
        Poll::Ready(result) => result,
        Poll::Pending => panic!("the call waited instead of ending at once"),
    }
}

pub fn panics_when_polled(pending: &mut Call<'_>) -> bool {
    catch_unwind(AssertUnwindSafe(|| poll_once(pending))).is_err()
}

pub async fn text_of(pending: Call<'_>) -> String {
    pending.await.expect("token").expose().to_owned()
}

pub fn alive_tasks() -> usize {
    tokio::runtime::Handle::current()
        .metrics()
        .num_alive_tasks()
}
