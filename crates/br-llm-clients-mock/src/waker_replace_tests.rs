use std::sync::Arc;

use crate::test_support::*;
use crate::waker_support::subjects;

#[test]
fn given_a_pending_future_when_repolled_with_another_waker_then_only_the_latest_waker_is_woken() {
    for mut subject in subjects() {
        let first = Arc::new(WakeCount::default());
        let second = Arc::new(WakeCount::default());
        assert!(
            poll_with(&mut subject.future.as_mut(), &first.waker()).is_pending(),
            "{}",
            subject.name
        );
        assert!(
            poll_with(&mut subject.future.as_mut(), &second.waker()).is_pending(),
            "{}",
            subject.name
        );
        (subject.fire)();
        assert_eq!(first.count(), 0, "{}", subject.name);
        assert_eq!(second.count(), 1, "{}", subject.name);
    }
}

#[test]
fn given_a_pending_future_when_repolled_with_the_same_waker_then_it_is_woken_once() {
    for mut subject in subjects() {
        let counter = Arc::new(WakeCount::default());
        let waker = counter.waker();
        for _ in 0..3 {
            assert!(poll_with(&mut subject.future.as_mut(), &waker).is_pending());
        }
        (subject.fire)();
        assert_eq!(counter.count(), 1, "{}", subject.name);
    }
}
