pub(crate) const TURNS: usize = 1_000_000;

pub(crate) async fn within_turns(ready: impl Fn() -> bool, missing: impl Fn() -> String) {
    for _ in 0..TURNS {
        if ready() {
            return;
        }
        tokio::task::yield_now().await;
    }
    panic!("after {TURNS} turns of the runtime: {}", missing());
}

pub(crate) fn within_spins(ready: impl Fn() -> bool, missing: impl Fn() -> String) {
    for _ in 0..TURNS {
        if ready() {
            return;
        }
        std::thread::yield_now();
    }
    panic!("after {TURNS} spins of the thread: {}", missing());
}

#[cfg(test)]
mod tests {
    use std::any::Any;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::{TURNS, within_spins, within_turns};

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
        let ended = tokio::spawn(within_turns(
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
            format!("after {TURNS} turns of the runtime: the wake is missing")
        );
        assert_eq!(calls.load(Ordering::SeqCst), TURNS);
    }

    #[test]
    fn given_a_condition_that_comes_true_then_the_spins_stop_at_once() {
        let calls = AtomicUsize::new(0);
        within_spins(
            || calls.fetch_add(1, Ordering::SeqCst) == 2,
            || "never".to_owned(),
        );
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn given_a_condition_that_never_comes_true_then_the_spins_end_after_the_bound_in_a_panic_that_says_what_is_missing()
     {
        let calls = AtomicUsize::new(0);
        let ended = catch_unwind(AssertUnwindSafe(|| {
            within_spins(
                || {
                    calls.fetch_add(1, Ordering::SeqCst);
                    false
                },
                || "the wake is missing".to_owned(),
            );
        }))
        .expect_err("the wait ends in a panic");
        assert_eq!(
            text_of(&*ended),
            format!("after {TURNS} spins of the thread: the wake is missing")
        );
        assert_eq!(calls.load(Ordering::SeqCst), TURNS);
    }
}
