use std::sync::Arc;
use std::task::Poll;
use std::time::Duration;

use br_llm_clients_common_http::{AuthorizerError, Secret};
use tokio::sync::Notify;
use tokio::time::{Instant, advance};

use crate::support::source::{Scripted, Step};
use crate::support::{Call, at_once, cache_of, call, parked, poll_once};

fn timed_out() -> AuthorizerError {
    AuthorizerError::fetch_failed("the token fetch timed out")
}

fn ready(pending: &mut Call<'_>) -> Result<Secret, AuthorizerError> {
    match poll_once(pending) {
        Poll::Ready(result) => result,
        Poll::Pending => panic!("the call is still waiting"),
    }
}

fn text(pending: &mut Call<'_>) -> String {
    ready(pending).expect("token").expose().to_owned()
}

#[tokio::test(start_paused = true)]
async fn given_a_leader_that_is_never_polled_again_then_a_waiter_fails_after_exactly_30_seconds() {
    let source = Scripted::new(vec![Step::hang(), Step::secs("new", 1000)]);
    let cache = cache_of(&source);
    let _stale = parked(&cache);
    let mut waiter = parked(&cache);
    advance(Duration::from_millis(29_999)).await;
    assert!(poll_once(&mut waiter).is_pending());
    advance(Duration::from_millis(1)).await;
    assert_eq!(ready(&mut waiter).expect_err("timed out"), timed_out());
    assert_eq!(source.calls(), 1);
    assert_eq!(at_once(&cache).expect("token").expose(), "new");
    assert_eq!(source.calls(), 2);
}

#[tokio::test(start_paused = true)]
async fn given_a_leader_that_is_never_polled_again_when_a_call_comes_after_30_seconds_then_it_leads_a_new_fetch()
 {
    let source = Scripted::new(vec![Step::hang(), Step::secs("new", 1000)]);
    let cache = cache_of(&source);
    let _stale = parked(&cache);
    advance(Duration::from_millis(29_999)).await;
    let _waiter = parked(&cache);
    advance(Duration::from_millis(1)).await;
    assert_eq!(at_once(&cache).expect("token").expose(), "new");
    assert_eq!(source.calls(), 2);
}

#[tokio::test(start_paused = true)]
async fn given_a_stale_leader_that_ends_later_then_it_changes_nothing_of_the_newer_fetch() {
    let old_gate = Arc::new(Notify::new());
    let new_gate = Arc::new(Notify::new());
    let source = Scripted::new(vec![
        Step::secs("stale", 1000).gated(&old_gate),
        Step::secs("new", 1000).gated(&new_gate),
    ]);
    let cache = cache_of(&source);
    let mut stale = parked(&cache);
    let mut expired = parked(&cache);
    advance(Duration::from_secs(30)).await;
    assert_eq!(ready(&mut expired).expect_err("timed out"), timed_out());
    let mut newer = parked(&cache);
    let mut waiter = parked(&cache);
    assert_eq!(source.calls(), 2);
    old_gate.notify_one();
    assert_eq!(ready(&mut stale).expect_err("late"), timed_out());
    assert!(poll_once(&mut call(&cache)).is_pending());
    assert!(poll_once(&mut waiter).is_pending());
    assert!(poll_once(&mut newer).is_pending());
    new_gate.notify_one();
    assert_eq!(text(&mut newer), "new");
    assert_eq!(text(&mut waiter), "new");
    assert_eq!(at_once(&cache).expect("token").expose(), "new");
    assert_eq!(source.calls(), 2);
}

#[tokio::test(start_paused = true)]
async fn given_a_leader_dropped_after_29_seconds_then_a_waiter_leads_a_new_fetch_with_its_own_30_seconds()
 {
    let source = Scripted::new(vec![Step::hang(), Step::hang()]);
    let cache = cache_of(&source);
    let start = Instant::now();
    let leader = parked(&cache);
    let mut waiter = parked(&cache);
    advance(Duration::from_secs(29)).await;
    drop(leader);
    assert!(poll_once(&mut waiter).is_pending());
    assert_eq!(source.calls(), 2);
    assert_eq!(waiter.await.expect_err("timed out"), timed_out());
    assert_eq!(start.elapsed(), Duration::from_secs(59));
}

#[tokio::test(start_paused = true)]
async fn given_a_leader_whose_result_comes_at_or_after_30_seconds_then_its_call_times_out_and_nothing_is_kept()
 {
    for late in [Duration::from_secs(30), Duration::from_secs(31)] {
        let gate = Arc::new(Notify::new());
        let source = Scripted::new(vec![
            Step::secs("late", 1000).gated(&gate),
            Step::secs("new", 1000),
        ]);
        let cache = cache_of(&source);
        let mut leader = parked(&cache);
        advance(late).await;
        gate.notify_one();
        assert_eq!(
            ready(&mut leader).expect_err("late"),
            timed_out(),
            "{late:?}"
        );
        assert_eq!(source.calls(), 1);
        assert_eq!(at_once(&cache).expect("token").expose(), "new");
        assert_eq!(source.calls(), 2);
    }
}

#[tokio::test(start_paused = true)]
async fn given_a_leader_with_a_usable_token_whose_result_comes_after_30_seconds_then_it_gives_the_usable_token()
 {
    let gate = Arc::new(Notify::new());
    let source = Scripted::new(vec![
        Step::secs("old", 1000),
        Step::secs("late", 1000).gated(&gate),
        Step::secs("new", 1000),
    ]);
    let cache = cache_of(&source);
    assert_eq!(at_once(&cache).expect("first").expose(), "old");
    advance(Duration::from_secs(800)).await;
    let mut leader = parked(&cache);
    advance(Duration::from_secs(31)).await;
    gate.notify_one();
    assert_eq!(text(&mut leader), "old");
    assert_eq!(source.calls(), 2);
    assert_eq!(at_once(&cache).expect("next").expose(), "new");
    assert_eq!(source.calls(), 3);
}

#[tokio::test(start_paused = true)]
async fn given_a_leader_whose_result_comes_just_before_30_seconds_then_the_token_is_kept() {
    let gate = Arc::new(Notify::new());
    let source = Scripted::new(vec![Step::secs("t", 1000).gated(&gate)]);
    let cache = cache_of(&source);
    let mut leader = parked(&cache);
    advance(Duration::from_millis(29_999)).await;
    gate.notify_one();
    assert_eq!(text(&mut leader), "t");
    assert_eq!(at_once(&cache).expect("kept").expose(), "t");
    assert_eq!(source.calls(), 1);
}
