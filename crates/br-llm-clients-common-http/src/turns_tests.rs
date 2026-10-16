use std::any::Any;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::turns::{within_turns, within_turns_of};

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
