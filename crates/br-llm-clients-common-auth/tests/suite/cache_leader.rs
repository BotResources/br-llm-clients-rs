use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::Poll;
use std::time::Duration;

use br_llm_clients_common_auth::{AccessToken, TokenCache, TokenSource};
use br_llm_clients_common_http::{AuthorizerError, BoxFuture, Secret};
use tokio::sync::Notify;

use crate::support::source::{Scripted, Step};
use crate::support::{at_once, cache_of, call, panics_when_polled, parked, poll_once, text_of};

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
                Some(Duration::from_secs(1000)),
            ))
        })
    }
}

#[tokio::test(start_paused = true)]
async fn given_a_dropped_leader_when_calls_wait_then_one_waiter_leads_and_none_fails() {
    let gate = Arc::new(Notify::new());
    let source = Scripted::new(vec![
        Step::secs("lost", 1000).gated(&gate),
        Step::secs("second", 1000).gated(&gate),
    ]);
    let cache = cache_of(&source);
    let leader = parked(&cache);
    let mut waiters: Vec<_> = (0..3).map(|_| parked(&cache)).collect();
    assert_eq!(source.calls(), 1);
    drop(leader);
    let mut first = waiters.remove(0);
    assert!(poll_once(&mut first).is_pending());
    assert_eq!(source.calls(), 2);
    for waiter in &mut waiters {
        assert!(poll_once(waiter).is_pending());
    }
    assert_eq!(source.calls(), 2);
    gate.notify_one();
    assert_eq!(text_of(first).await, "second");
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
    drop(parked(&cache));
    assert_eq!(at_once(&cache).expect("token").expose(), "second");
    assert_eq!(source.calls(), 2);
}

#[tokio::test(start_paused = true)]
async fn given_a_source_that_panics_when_calls_wait_then_the_panic_reaches_the_leader_only() {
    let gate = Arc::new(Notify::new());
    let source = Scripted::new(vec![Step::panic().gated(&gate), Step::secs("second", 1000)]);
    let cache = cache_of(&source);
    let mut leader = parked(&cache);
    let mut waiters: Vec<_> = (0..3).map(|_| parked(&cache)).collect();
    gate.notify_one();
    assert!(panics_when_polled(&mut leader));
    drop(leader);
    for waiter in &mut waiters {
        match poll_once(waiter) {
            Poll::Ready(result) => assert_eq!(result.expect("token").expose(), "second"),
            Poll::Pending => panic!("a waiter did not take over after the panic"),
        }
    }
    assert_eq!(source.calls(), 2);
}

#[tokio::test(start_paused = true)]
async fn given_a_source_that_panics_before_its_future_when_called_again_then_the_cache_still_works()
{
    let source = Arc::new(PanicsBeforeTheFuture {
        calls: AtomicUsize::new(0),
    });
    let cache = TokenCache::new(source.clone());
    let mut leader = call(&cache);
    assert!(panics_when_polled(&mut leader));
    drop(leader);
    assert_eq!(at_once(&cache).expect("token").expose(), "again");
    assert_eq!(source.calls.load(Ordering::SeqCst), 2);
}
