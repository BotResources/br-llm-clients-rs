use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use br_llm_clients_common_auth::{AccessToken, StaticToken, TokenCache, TokenSource};
use br_llm_clients_common_http::{AuthorizerError, BoxFuture, Secret};
use tokio::time::advance;

use crate::support::source::{Scripted, Step};
use crate::support::{alive_tasks, cache_of};

struct Bare;

impl TokenSource for Bare {
    fn fetch(&self) -> BoxFuture<'_, Result<AccessToken, AuthorizerError>> {
        Box::pin(async move { Ok(AccessToken::new(Secret::new("t").expect("secret"), None)) })
    }
}

fn ms(value: u64) -> Duration {
    Duration::from_millis(value)
}

async fn calls_after(validity: u64, elapsed: Duration) -> usize {
    let source = Scripted::new(vec![Step::secs("t", validity)]);
    let cache = cache_of(&source);
    cache.token().await.expect("first");
    advance(elapsed).await;
    cache.token().await.expect("second");
    source.calls()
}

#[tokio::test(start_paused = true)]
async fn given_a_long_validity_when_the_time_left_is_above_300_seconds_then_the_token_is_reused() {
    assert_eq!(calls_after(1000, ms(699_999)).await, 1);
}

#[tokio::test(start_paused = true)]
async fn given_a_long_validity_when_the_time_left_is_exactly_300_seconds_then_a_new_fetch_starts() {
    assert_eq!(calls_after(1000, Duration::from_secs(700)).await, 2);
}

#[tokio::test(start_paused = true)]
async fn given_a_validity_of_100_seconds_when_half_is_left_then_a_new_fetch_starts() {
    assert_eq!(calls_after(100, ms(49_999)).await, 1);
    assert_eq!(calls_after(100, Duration::from_secs(50)).await, 2);
}

#[tokio::test(start_paused = true)]
async fn given_a_validity_just_under_600_seconds_then_the_skew_is_half_the_validity() {
    assert_eq!(calls_after(599, ms(299_200)).await, 1);
    assert_eq!(calls_after(599, ms(299_600)).await, 2);
}

#[tokio::test(start_paused = true)]
async fn given_a_validity_of_600_seconds_then_the_skew_is_300_seconds() {
    assert_eq!(calls_after(600, ms(299_999)).await, 1);
    assert_eq!(calls_after(600, Duration::from_secs(300)).await, 2);
}

#[tokio::test(start_paused = true)]
async fn given_a_validity_just_over_600_seconds_then_the_skew_is_300_seconds() {
    assert_eq!(calls_after(601, ms(300_900)).await, 1);
    assert_eq!(calls_after(601, ms(301_100)).await, 2);
}

#[tokio::test(start_paused = true)]
async fn given_a_slow_fetch_when_the_validity_is_counted_then_it_starts_when_the_fetch_started() {
    let source = Scripted::new(vec![Step::secs("t", 1000).after(Duration::from_secs(20))]);
    let cache = cache_of(&source);
    cache.token().await.expect("first");
    advance(ms(679_999)).await;
    cache.token().await.expect("second");
    assert_eq!(source.calls(), 1);
    advance(ms(1)).await;
    cache.token().await.expect("third");
    assert_eq!(source.calls(), 2);
}

#[tokio::test(start_paused = true)]
async fn given_a_validity_above_24_hours_then_it_counts_as_24_hours() {
    let day = 24 * 60 * 60;
    assert_eq!(
        calls_after(30 * day, Duration::from_secs(day - 301)).await,
        1
    );
    assert_eq!(
        calls_after(30 * day, Duration::from_secs(day - 300)).await,
        2
    );
}

#[tokio::test(start_paused = true)]
async fn given_the_largest_validity_when_the_cache_keeps_it_then_nothing_overflows() {
    let source = Scripted::new(vec![Step::token("t", Some(Duration::MAX))]);
    let cache = cache_of(&source);
    cache.token().await.expect("first");
    advance(Duration::from_secs(60)).await;
    cache.token().await.expect("second");
    assert_eq!(source.calls(), 1);
}

#[tokio::test(start_paused = true)]
async fn given_no_validity_when_called_again_then_the_source_is_asked_each_time() {
    let source = Scripted::new(vec![Step::token("t", None)]);
    let cache = cache_of(&source);
    for expected in 1..=3 {
        assert_eq!(cache.token().await.expect("token").expose(), "t");
        assert_eq!(source.calls(), expected);
    }
}

#[tokio::test(start_paused = true)]
async fn given_a_validity_of_zero_when_called_again_then_the_source_is_asked_each_time() {
    let source = Scripted::new(vec![Step::token("t", Some(Duration::ZERO))]);
    let cache = cache_of(&source);
    for expected in 1..=3 {
        assert_eq!(cache.token().await.expect("token").expose(), "t");
        assert_eq!(source.calls(), expected);
    }
}

struct Counting {
    inner: StaticToken,
    fetched: AtomicUsize,
}

impl TokenSource for Counting {
    fn fetch(&self) -> BoxFuture<'_, Result<AccessToken, AuthorizerError>> {
        self.fetched.fetch_add(1, Ordering::SeqCst);
        self.inner.fetch()
    }

    fn secrets(&self) -> Vec<Secret> {
        self.inner.secrets()
    }
}

#[tokio::test(start_paused = true)]
async fn given_a_static_token_when_called_again_then_it_has_no_validity_and_is_fetched_each_time() {
    let token = Secret::new("static-token-value").expect("secret");
    let source = Arc::new(Counting {
        inner: StaticToken::new(token),
        fetched: AtomicUsize::new(0),
    });
    let cache = TokenCache::new(source.clone());
    for expected in 1..=3 {
        assert_eq!(
            cache.token().await.expect("token").expose(),
            "static-token-value"
        );
        assert_eq!(source.fetched.load(Ordering::SeqCst), expected);
    }
    let secrets = cache.secrets();
    assert_eq!(secrets.len(), 1);
    assert_eq!(secrets[0].expose(), "static-token-value");
}

#[test]
fn given_a_new_cache_when_built_outside_a_runtime_then_it_makes_no_fetch() {
    let source = Scripted::new(vec![Step::secs("t", 100)]);
    let cache = cache_of(&source);
    let clone = cache.clone();
    assert_eq!(source.calls(), 0);
    drop((cache, clone));
    assert_eq!(source.calls(), 0);
}

#[tokio::test(start_paused = true)]
async fn given_a_cache_when_time_passes_then_no_fetch_starts_in_the_background() {
    let source = Scripted::new(vec![Step::secs("t", 100)]);
    let cache = cache_of(&source);
    assert_eq!(alive_tasks(), 0);
    cache.token().await.expect("first");
    assert_eq!(alive_tasks(), 0);
    advance(Duration::from_secs(10_000)).await;
    assert_eq!(alive_tasks(), 0);
    assert_eq!(source.calls(), 1);
}

#[tokio::test(start_paused = true)]
async fn given_two_clones_when_both_ask_then_they_share_the_token() {
    let source = Scripted::new(vec![Step::secs("t", 1000)]);
    let cache = cache_of(&source);
    let clone = cache.clone();
    cache.token().await.expect("first");
    clone.token().await.expect("second");
    assert_eq!(source.calls(), 1);
}

#[tokio::test(start_paused = true)]
async fn given_a_source_with_secrets_then_the_cache_gives_them_and_a_plain_source_gives_none() {
    let secret = Secret::new("client-secret-value").expect("secret");
    let holding = Scripted::holding(vec![Step::secs("t", 1)], vec![secret]);
    assert_eq!(
        cache_of(&holding).secrets()[0].expose(),
        "client-secret-value"
    );
    assert!(TokenCache::new(Arc::new(Bare)).secrets().is_empty());
}
