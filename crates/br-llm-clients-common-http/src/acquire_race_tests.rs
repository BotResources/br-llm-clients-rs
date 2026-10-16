use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::acquire_tests::{Outcome, Rig, outcome};
use crate::dialer::DialFailure;
use crate::transport::bump;

const WAITERS: usize = 64;
const ROUNDS: usize = 100;
const BUMPERS: usize = 2;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_wakes_for_unrelated_changes_while_a_dial_fails_then_no_waiter_misses_that_failure_and_none_dials_again()
 {
    let rig = Rig::announcing_server(1).await;
    let (full, _held) = rig.land(rig.dialer()).await;
    full.apply_settings(Some(1));
    let signal = rig
        .shared
        .pool
        .lock()
        .get(&rig.key)
        .map(|host| Arc::clone(&host.signal))
        .expect("host");
    let mut misses = 0usize;
    for _ in 0..ROUNDS {
        let capacity = rig.dialer();
        let waiters: Vec<_> = (0..WAITERS).map(|_| rig.waiter()).collect();
        rig.waiting(WAITERS).await;
        let stop = Arc::new(AtomicBool::new(false));
        let bumpers: Vec<_> = (0..BUMPERS)
            .map(|_| {
                let (signal, stop) = (Arc::clone(&signal), Arc::clone(&stop));
                std::thread::spawn(move || {
                    while !stop.load(Ordering::Relaxed) {
                        bump(&signal);
                    }
                })
            })
            .collect();
        tokio::time::sleep(Duration::from_millis(3)).await;
        capacity.fail(DialFailure::Connect("refused".to_owned()));
        for waiter in waiters {
            if !matches!(outcome(waiter).await, Outcome::Failed(_)) {
                misses += 1;
            }
        }
        stop.store(true, Ordering::Relaxed);
        for bumper in bumpers {
            bumper.join().expect("bumper");
        }
        if misses > 0 {
            break;
        }
    }
    assert_eq!(misses, 0, "waiters that missed the failure of their dial");
    assert_eq!(rig.accepted(), 1, "a waiter that missed it dialed again");
}
