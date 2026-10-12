use std::sync::Arc;

use crate::test_support::*;
use crate::waker_support::{probing_waker, subjects, within_deadline};

#[test]
fn given_a_pending_future_when_dropped_then_its_waker_is_released() {
    for mut subject in subjects() {
        let counter = Arc::new(WakeCount::default());
        let waker = counter.waker();
        let baseline = Arc::strong_count(&counter);
        assert!(poll_with(&mut subject.future.as_mut(), &waker).is_pending());
        assert!(Arc::strong_count(&counter) > baseline, "{}", subject.name);
        drop(subject.future);
        assert_eq!(Arc::strong_count(&counter), baseline, "{}", subject.name);
    }
}

#[test]
fn given_a_pending_future_when_repolled_with_another_waker_then_the_first_waker_is_released() {
    for mut subject in subjects() {
        let first = Arc::new(WakeCount::default());
        let second = Arc::new(WakeCount::default());
        let first_waker = first.waker();
        let second_waker = second.waker();
        let first_baseline = Arc::strong_count(&first);
        let second_baseline = Arc::strong_count(&second);
        assert!(poll_with(&mut subject.future.as_mut(), &first_waker).is_pending());
        assert!(poll_with(&mut subject.future.as_mut(), &second_waker).is_pending());
        assert_eq!(
            Arc::strong_count(&first),
            first_baseline,
            "{}",
            subject.name
        );
        assert!(
            Arc::strong_count(&second) > second_baseline,
            "{}",
            subject.name
        );
    }
}

#[test]
fn given_a_woken_future_when_the_wake_has_run_then_its_waker_is_released() {
    for mut subject in subjects() {
        let counter = Arc::new(WakeCount::default());
        let waker = counter.waker();
        let baseline = Arc::strong_count(&counter);
        assert!(poll_with(&mut subject.future.as_mut(), &waker).is_pending());
        (subject.fire)();
        assert_eq!(Arc::strong_count(&counter), baseline, "{}", subject.name);
    }
}

#[test]
fn given_a_waker_whose_destructor_uses_the_same_object_when_the_future_is_repolled_then_there_is_no_deadlock()
 {
    for subject in subjects() {
        within_deadline(move || {
            let mut subject = subject;
            let reentrant = probing_waker(Arc::clone(&subject.touch));
            let counter = Arc::new(WakeCount::default());
            assert!(poll_with(&mut subject.future.as_mut(), &reentrant).is_pending());
            drop(reentrant);
            assert!(poll_with(&mut subject.future.as_mut(), &counter.waker()).is_pending());
        });
    }
}

#[test]
fn given_a_waker_whose_destructor_uses_the_same_object_when_the_future_is_dropped_then_there_is_no_deadlock()
 {
    for subject in subjects() {
        within_deadline(move || {
            let mut subject = subject;
            let reentrant = probing_waker(Arc::clone(&subject.touch));
            assert!(poll_with(&mut subject.future.as_mut(), &reentrant).is_pending());
            drop(reentrant);
            drop(subject.future);
        });
    }
}

#[test]
fn given_a_waker_whose_destructor_uses_the_same_object_when_the_wake_runs_then_there_is_no_deadlock()
 {
    for subject in subjects() {
        within_deadline(move || {
            let mut subject = subject;
            let reentrant = probing_waker(Arc::clone(&subject.touch));
            assert!(poll_with(&mut subject.future.as_mut(), &reentrant).is_pending());
            drop(reentrant);
            (subject.fire)();
        });
    }
}
