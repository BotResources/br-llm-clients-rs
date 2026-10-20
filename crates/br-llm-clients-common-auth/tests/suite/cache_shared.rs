use std::sync::Arc;
use std::time::Duration;

use br_llm_clients_common_http::AuthorizerError;
use tokio::sync::Notify;
use tokio::time::advance;

use crate::support::source::{Scripted, Step};
use crate::support::{cache_of, settle, spawn_call, text_of};

#[tokio::test(start_paused = true)]
async fn given_many_calls_without_a_token_when_they_arrive_together_then_one_fetch_serves_all() {
    let gate = Arc::new(Notify::new());
    let source = Scripted::new(vec![Step::secs("t1", 1000).gated(&gate)]);
    let cache = cache_of(&source);
    let calls: Vec<_> = (0..8).map(|_| spawn_call(&cache)).collect();
    settle().await;
    assert_eq!(source.calls(), 1);
    assert!(calls.iter().all(|call| !call.is_finished()));
    gate.notify_one();
    for call in calls {
        assert_eq!(text_of(call).await, "t1");
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
    let calls: Vec<_> = (0..4).map(|_| spawn_call(&cache)).collect();
    settle().await;
    gate.notify_one();
    for call in calls {
        assert_eq!(call.await.expect("task").expect_err("error"), error);
    }
    assert_eq!(source.calls(), 1);
    assert_eq!(cache.token().await.expect("token").expose(), "t2");
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
    assert_eq!(cache.token().await.expect("first").expose(), "old");
    advance(Duration::from_secs(800)).await;
    let leader = spawn_call(&cache);
    settle().await;
    assert_eq!(source.calls(), 2);
    assert_eq!(cache.token().await.expect("second").expose(), "old");
    assert!(!leader.is_finished());
    gate.notify_one();
    assert_eq!(text_of(leader).await, "new");
    assert_eq!(cache.token().await.expect("third").expose(), "new");
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
    cache.token().await.expect("first");
    advance(Duration::from_secs(800)).await;
    let leader = spawn_call(&cache);
    settle().await;
    assert!(!leader.is_finished());
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
    cache.token().await.expect("first");
    advance(Duration::from_secs(101)).await;
    let leader = spawn_call(&cache);
    let waiter = spawn_call(&cache);
    settle().await;
    assert!(!leader.is_finished());
    assert!(!waiter.is_finished());
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
    let calls: Vec<_> = (0..4).map(|_| spawn_call(&cache)).collect();
    settle().await;
    gate.notify_one();
    for call in calls {
        assert_eq!(text_of(call).await, "first");
    }
    assert_eq!(source.calls(), 1);
    assert_eq!(cache.token().await.expect("next").expose(), "second");
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
    cache.token().await.expect("first");
    advance(Duration::from_secs(800)).await;
    assert_eq!(cache.token().await.expect("second").expose(), "rotating");
    assert_eq!(cache.token().await.expect_err("third"), error);
}
