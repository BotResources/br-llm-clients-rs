use std::any::Any;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use crate::turns::{within_time, within_turns, within_turns_of};

const SMALL_BOUND: usize = 1_000;

fn text_of(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .unwrap_or_default()
}

#[tokio::test]
async fn given_a_condition_that_comes_true_then_the_turns_stop_at_once() {
    let calls = AtomicUsize::new(0);
    within_turns(
        || calls.fetch_add(1, Ordering::SeqCst) == 2,
        || "never".to_owned(),
    )
    .await;
    assert_eq!(calls.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn given_a_condition_that_never_comes_true_then_the_turns_end_after_the_bound_in_a_panic_that_says_what_is_missing()
 {
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&calls);
    let ended = tokio::spawn(within_turns_of(
        SMALL_BOUND,
        move || {
            counted.fetch_add(1, Ordering::SeqCst);
            false
        },
        || "the wake is missing".to_owned(),
    ))
    .await
    .expect_err("the wait ends in a panic");
    assert_eq!(
        text_of(&*ended.into_panic()),
        format!("after {SMALL_BOUND} turns of the runtime: the wake is missing")
    );
    assert_eq!(calls.load(Ordering::SeqCst), SMALL_BOUND);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn given_a_condition_that_another_thread_makes_true_then_the_wait_in_time_ends_when_it_holds()
{
    let flag = Arc::new(AtomicUsize::new(0));
    let raised = Arc::clone(&flag);
    let raiser = tokio::spawn(async move { raised.store(1, Ordering::SeqCst) });
    within_time(
        Duration::from_secs(30),
        || flag.load(Ordering::SeqCst) == 1,
        || "never".to_owned(),
    )
    .await;
    raiser.await.expect("raiser");
}

#[tokio::test]
async fn given_a_condition_that_never_comes_true_then_the_wait_in_time_ends_after_the_limit_in_a_panic_that_says_what_is_missing()
 {
    let ended = tokio::spawn(within_time(
        Duration::from_millis(50),
        || false,
        || "the wake is missing".to_owned(),
    ))
    .await
    .expect_err("the wait ends in a panic");
    assert_eq!(
        text_of(&*ended.into_panic()),
        "after 50ms: the wake is missing"
    );
}
