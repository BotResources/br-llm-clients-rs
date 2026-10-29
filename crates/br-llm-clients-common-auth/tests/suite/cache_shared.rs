use std::sync::Arc;
use std::time::Duration;

use br_llm_clients_common_http::AuthorizerError;
use tokio::sync::Notify;
use tokio::time::advance;

use crate::support::source::{Scripted, Step};
use crate::support::{at_once, cache_of, call, parked, text_of};

#[tokio::test(start_paused = true)]
async fn given_many_calls_on_clones_without_a_token_when_they_arrive_together_then_one_fetch_serves_all()
 {
    let gate = Arc::new(Notify::new());
    let source = Scripted::new(vec![Step::secs("t1", 1000).gated(&gate)]);
    let cache = cache_of(&source);
    let clones: Vec<_> = (0..8).map(|_| cache.clone()).collect();
    let calls: Vec<_> = clones.iter().map(parked).collect();
    assert_eq!(source.calls(), 1);
    gate.notify_one();
    for pending in calls {
        assert_eq!(text_of(pending).await, "t1");
    }
    assert_eq!(source.calls(), 1);
}

#[tokio::test(start_paused = true)]
async fn given_a_failed_fetch_when_calls_wait_then_they_all_get_the_error_and_it_is_not_kept() {
    let gate = Arc::new(Notify::new());
    let error = AuthorizerError::unavailable("no credential");
    let source = Scripted::new(vec![
        Step::fail(error.clone()).gated(&gate),
        Step::secs("t2", 1000),
    ]);
    let cache = cache_of(&source);
    let calls: Vec<_> = (0..4).map(|_| parked(&cache)).collect();
    gate.notify_one();
    for pending in calls {
        assert_eq!(pending.await.expect_err("error"), error);
    }
    assert_eq!(source.calls(), 1);
    assert_eq!(text_of(call(&cache)).await, "t2");
    assert_eq!(source.calls(), 2);
}

#[tokio::test(start_paused = true)]
async fn given_a_usable_token_when_a_fetch_runs_then_a_call_gets_the_token_at_once() {
    let gate = Arc::new(Notify::new());
    let source = Scripted::new(vec![
        Step::secs("old", 1000),
        Step::secs("new", 1000).gated(&gate),
    ]);
    let cache = cache_of(&source);
    assert_eq!(text_of(call(&cache)).await, "old");
    advance(Duration::from_secs(800)).await;
    let leader = parked(&cache);
    assert_eq!(source.calls(), 2);
    assert_eq!(at_once(&cache).expect("second").expose(), "old");
    gate.notify_one();
    assert_eq!(text_of(leader).await, "new");
    assert_eq!(at_once(&cache).expect("third").expose(), "new");
    assert_eq!(source.calls(), 2);
}

#[tokio::test(start_paused = true)]
async fn given_a_fetch_that_runs_when_the_time_left_reaches_zero_then_a_call_waits_for_the_fetch() {
    let gate = Arc::new(Notify::new());
    let source = Scripted::new(vec![
        Step::secs("old", 100),
        Step::secs("new", 1000).gated(&gate),
    ]);
    let cache = cache_of(&source);
    assert_eq!(text_of(call(&cache)).await, "old");
    advance(Duration::from_millis(99_999)).await;
    let leader = parked(&cache);
    assert_eq!(at_once(&cache).expect("usable").expose(), "old");
    advance(Duration::from_millis(1)).await;
    let waiter = parked(&cache);
    assert_eq!(source.calls(), 2);
    gate.notify_one();
    assert_eq!(text_of(leader).await, "new");
    assert_eq!(text_of(waiter).await, "new");
    assert_eq!(source.calls(), 2);
}

#[tokio::test(start_paused = true)]
async fn given_a_usable_token_when_a_call_leads_then_it_waits_for_its_own_fetch() {
    let gate = Arc::new(Notify::new());
    let source = Scripted::new(vec![
        Step::secs("old", 1000),
        Step::secs("new", 1000).gated(&gate),
    ]);
    let cache = cache_of(&source);
    assert_eq!(text_of(call(&cache)).await, "old");
    advance(Duration::from_secs(800)).await;
    let leader = parked(&cache);
    gate.notify_one();
    assert_eq!(text_of(leader).await, "new");
}

#[tokio::test(start_paused = true)]
async fn given_no_usable_token_when_a_fetch_runs_then_calls_wait_and_get_its_result() {
    let gate = Arc::new(Notify::new());
    let source = Scripted::new(vec![
        Step::secs("old", 100),
        Step::secs("new", 1000).gated(&gate),
    ]);
    let cache = cache_of(&source);
    assert_eq!(text_of(call(&cache)).await, "old");
    advance(Duration::from_secs(101)).await;
    let leader = parked(&cache);
    let waiter = parked(&cache);
    assert_eq!(source.calls(), 2);
    gate.notify_one();
    assert_eq!(text_of(leader).await, "new");
    assert_eq!(text_of(waiter).await, "new");
    assert_eq!(source.calls(), 2);
}

#[tokio::test(start_paused = true)]
async fn given_a_token_with_no_validity_when_calls_wait_then_they_get_it_and_the_next_call_fetches()
{
    let gate = Arc::new(Notify::new());
    let source = Scripted::new(vec![
        Step::token("first", None).gated(&gate),
        Step::token("second", None),
    ]);
    let cache = cache_of(&source);
    let calls: Vec<_> = (0..4).map(|_| parked(&cache)).collect();
    gate.notify_one();
    for pending in calls {
        assert_eq!(text_of(pending).await, "first");
    }
    assert_eq!(source.calls(), 1);
    assert_eq!(text_of(call(&cache)).await, "second");
    assert_eq!(source.calls(), 2);
}

#[tokio::test(start_paused = true)]
async fn given_a_new_token_with_no_validity_when_a_later_fetch_fails_then_the_old_token_is_gone() {
    let error = AuthorizerError::fetch_failed("down");
    let source = Scripted::new(vec![
        Step::secs("old", 1000),
        Step::token("rotating", None),
        Step::fail(error.clone()),
    ]);
    let cache = cache_of(&source);
    assert_eq!(text_of(call(&cache)).await, "old");
    advance(Duration::from_secs(800)).await;
    assert_eq!(text_of(call(&cache)).await, "rotating");
    assert_eq!(call(&cache).await.expect_err("third"), error);
}
