use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use br_llm_clients_common_auth::{AccessToken, TokenCache, TokenSource};
use br_llm_clients_common_http::{AuthorizerError, BoxFuture, Secret};
use tokio::sync::Notify;

use crate::support::source::{Scripted, Step};
use crate::support::{cache_of, settle, spawn_call, text_of};

struct PanicsBeforeTheFuture {
    calls: AtomicUsize,
}

impl TokenSource for PanicsBeforeTheFuture {
    fn fetch(&self) -> BoxFuture<'_, Result<AccessToken, AuthorizerError>> {
        if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
            panic!("the source panics before it gives a future");
        }
        Box::pin(async move {
            Ok(AccessToken::new(
                Secret::new("again").expect("secret"),
                Some(std::time::Duration::from_secs(1000)),
            ))
        })
    }
}

#[tokio::test(start_paused = true)]
async fn given_a_dropped_leader_when_calls_wait_then_one_waiter_leads_and_none_fails() {
    let gate = Arc::new(Notify::new());
    let source = Scripted::new(vec![
        Step::secs("lost", 1000).gated(&gate),
        Step::secs("second", 1000),
    ]);
    let cache = cache_of(&source);
    let leader = spawn_call(&cache);
    let waiters: Vec<_> = (0..3).map(|_| spawn_call(&cache)).collect();
    settle().await;
    assert_eq!(source.calls(), 1);
    leader.abort();
    assert!(leader.await.expect_err("aborted").is_cancelled());
    for waiter in waiters {
        assert_eq!(text_of(waiter).await, "second");
    }
    assert_eq!(source.calls(), 2);
}

#[tokio::test(start_paused = true)]
async fn given_a_dropped_leader_and_no_waiter_when_called_again_then_no_lock_is_held() {
    let gate = Arc::new(Notify::new());
    let source = Scripted::new(vec![
        Step::secs("lost", 1000).gated(&gate),
        Step::secs("second", 1000),
    ]);
    let cache = cache_of(&source);
    let leader = spawn_call(&cache);
    settle().await;
    leader.abort();
    let _ = leader.await;
    assert_eq!(cache.token().await.expect("token").expose(), "second");
    assert_eq!(source.calls(), 2);
}

#[tokio::test(start_paused = true)]
async fn given_a_source_that_panics_when_calls_wait_then_the_panic_reaches_the_leader_only() {
    let gate = Arc::new(Notify::new());
    let source = Scripted::new(vec![Step::panic().gated(&gate), Step::secs("second", 1000)]);
    let cache = cache_of(&source);
    let leader = spawn_call(&cache);
    settle().await;
    let waiters: Vec<_> = (0..3).map(|_| spawn_call(&cache)).collect();
    settle().await;
    gate.notify_one();
    assert!(leader.await.expect_err("panicked").is_panic());
    for waiter in waiters {
        assert_eq!(text_of(waiter).await, "second");
    }
    assert_eq!(source.calls(), 2);
}

#[tokio::test(start_paused = true)]
async fn given_a_source_that_panics_before_its_future_when_called_again_then_the_cache_still_works()
{
    let source = Arc::new(PanicsBeforeTheFuture {
        calls: AtomicUsize::new(0),
    });
    let cache = TokenCache::new(source);
    let leader = spawn_call(&cache);
    assert!(leader.await.expect_err("panicked").is_panic());
    assert_eq!(cache.token().await.expect("token").expose(), "again");
}
