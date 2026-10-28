use std::time::Duration;

use br_llm_clients_common_http::AuthorizerError;
use tokio::time::{Instant, advance};

use crate::support::cache_of;
use crate::support::source::{Scripted, Step};

fn timed_out() -> AuthorizerError {
    AuthorizerError::fetch_failed("the token fetch timed out")
}

#[tokio::test(start_paused = true)]
async fn given_a_failed_fetch_and_no_token_then_the_error_of_the_source_is_given_unchanged() {
    for error in [
        AuthorizerError::unavailable("no credential"),
        AuthorizerError::fetch_failed("down"),
    ] {
        let source = Scripted::new(vec![Step::fail(error.clone())]);
        assert_eq!(cache_of(&source).token().await.expect_err("error"), error);
    }
}

#[tokio::test(start_paused = true)]
async fn given_a_failed_fetch_when_called_again_then_the_error_was_not_kept() {
    let source = Scripted::new(vec![
        Step::fail(AuthorizerError::fetch_failed("down")),
        Step::secs("t", 1000),
    ]);
    let cache = cache_of(&source);
    cache.token().await.expect_err("first");
    assert_eq!(cache.token().await.expect("second").expose(), "t");
    assert_eq!(source.calls(), 2);
}

#[tokio::test(start_paused = true)]
async fn given_a_usable_token_when_the_fetch_fails_then_the_leader_gives_the_token_not_the_error() {
    let source = Scripted::new(vec![
        Step::secs("old", 1000),
        Step::fail(AuthorizerError::fetch_failed("down")),
    ]);
    let cache = cache_of(&source);
    cache.token().await.expect("first");
    advance(Duration::from_secs(800)).await;
    assert_eq!(cache.token().await.expect("second").expose(), "old");
    assert_eq!(cache.token().await.expect("third").expose(), "old");
    assert_eq!(source.calls(), 3);
}

#[tokio::test(start_paused = true)]
async fn given_a_token_that_ends_during_a_failing_fetch_then_the_leader_gets_the_error() {
    let error = AuthorizerError::fetch_failed("down");
    let source = Scripted::new(vec![
        Step::secs("old", 100),
        Step::fail(error.clone()).after(Duration::from_secs(25)),
    ]);
    let cache = cache_of(&source);
    cache.token().await.expect("first");
    advance(Duration::from_secs(80)).await;
    assert_eq!(cache.token().await.expect_err("second"), error);
}

#[tokio::test(start_paused = true)]
async fn given_a_fetch_that_never_ends_then_it_fails_after_exactly_30_seconds_with_a_fixed_text() {
    let source = Scripted::new(vec![Step::hang(), Step::secs("t", 1000)]);
    let cache = cache_of(&source);
    let start = Instant::now();
    assert_eq!(cache.token().await.expect_err("timeout"), timed_out());
    assert_eq!(start.elapsed(), Duration::from_secs(30));
    assert_eq!(cache.token().await.expect("next").expose(), "t");
}

#[tokio::test(start_paused = true)]
async fn given_a_fetch_of_29_seconds_then_it_is_not_cut() {
    let source = Scripted::new(vec![Step::secs("t", 1000).after(Duration::from_secs(29))]);
    assert_eq!(
        cache_of(&source).token().await.expect("token").expose(),
        "t"
    );
}

#[tokio::test(start_paused = true)]
async fn given_a_timed_out_fetch_when_calls_wait_then_they_share_the_timeout() {
    let source = Scripted::new(vec![Step::hang()]);
    let cache = cache_of(&source);
    let calls: Vec<_> = (0..3).map(|_| crate::support::spawn_call(&cache)).collect();
    for call in calls {
        assert_eq!(call.await.expect("task").expect_err("error"), timed_out());
    }
    assert_eq!(source.calls(), 1);
}

#[tokio::test(start_paused = true)]
async fn given_a_usable_token_when_the_fetch_times_out_then_the_leader_gives_the_token() {
    let source = Scripted::new(vec![Step::secs("old", 1000), Step::hang()]);
    let cache = cache_of(&source);
    cache.token().await.expect("first");
    advance(Duration::from_secs(800)).await;
    assert_eq!(cache.token().await.expect("second").expose(), "old");
}
