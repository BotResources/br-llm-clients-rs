use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use crate::sync::{Condition, Wait, Waiters, lock};
use crate::test_support::*;

#[derive(Default)]
struct Flag {
    raised: bool,
    waiters: Waiters,
}

struct Counting {
    flag: Arc<Mutex<Flag>>,
    locks: Arc<AtomicUsize>,
}

impl Condition for Counting {
    type State = Flag;

    fn lock(&self) -> MutexGuard<'_, Flag> {
        self.locks.fetch_add(1, Ordering::SeqCst);
        lock(&self.flag)
    }

    fn holds(&self, state: &Flag) -> bool {
        state.raised
    }

    fn waiters(state: &mut Flag) -> &mut Waiters {
        &mut state.waiters
    }
}

fn counting() -> (Wait<Counting>, Arc<Mutex<Flag>>, Arc<AtomicUsize>) {
    let flag = Arc::new(Mutex::new(Flag::default()));
    let locks = Arc::new(AtomicUsize::new(0));
    let wait = Wait::new(Counting {
        flag: Arc::clone(&flag),
        locks: Arc::clone(&locks),
    });
    (wait, flag, locks)
}

#[test]
fn given_a_condition_that_does_not_hold_when_polled_then_check_and_register_share_one_lock() {
    let (wait, _flag, locks) = counting();
    let mut wait = Box::pin(wait);
    assert!(poll_once(&mut wait.as_mut()).is_pending());
    assert_eq!(locks.load(Ordering::SeqCst), 1);
    assert!(poll_once(&mut wait.as_mut()).is_pending());
    assert_eq!(locks.load(Ordering::SeqCst), 2);
}

#[test]
fn given_a_condition_that_holds_when_polled_then_check_and_cancel_share_one_lock() {
    let (wait, flag, locks) = counting();
    let mut wait = Box::pin(wait);
    assert!(poll_once(&mut wait.as_mut()).is_pending());
    lock(&flag).raised = true;
    assert!(poll_once(&mut wait.as_mut()).is_ready());
    assert_eq!(locks.load(Ordering::SeqCst), 2);
}
