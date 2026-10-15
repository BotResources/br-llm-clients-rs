use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tokio::sync::watch;

use crate::endpoint::Endpoint;
use crate::h2state::H2State;
use crate::idle::IdleClock;
use crate::pool::{HostHandle, Pool, PoolKey};

fn bare_state() -> Arc<H2State> {
    let pool = Arc::new(Pool::new(Duration::from_secs(30)));
    let key = PoolKey::of(&Endpoint::https("https://h.example/").expect("endpoint"));
    let host = HostHandle {
        signal: Arc::new(watch::channel(0u64).0),
        pool: Arc::downgrade(&pool),
        key,
    };
    let (idle, _) = IdleClock::new(Duration::from_secs(30));
    H2State::new(idle, host)
}

fn state_with_limit(limit: u32) -> Arc<H2State> {
    let state = bare_state();
    state.apply_settings(Some(limit));
    state
}

#[test]
fn given_permits_taken_and_dropped_by_many_threads_then_the_idle_timer_is_never_armed_while_one_is_held()
 {
    let state = state_with_limit(1000);
    let finished = AtomicBool::new(false);
    let violations = std::thread::scope(|scope| {
        let watcher = scope.spawn(|| {
            let mut violations = 0usize;
            while !finished.load(Ordering::SeqCst) {
                if state.violates_idle_invariant() {
                    violations += 1;
                }
            }
            violations
        });
        let workers: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    for _ in 0..4000 {
                        if let Some(permit) = state.try_acquire() {
                            std::thread::sleep(Duration::from_micros(20));
                            drop(permit);
                        }
                    }
                })
            })
            .collect();
        for worker in workers {
            worker.join().expect("worker");
        }
        finished.store(true, Ordering::SeqCst);
        watcher.join().expect("watcher")
    });
    assert_eq!(violations, 0);
}

#[test]
fn given_a_send_taking_a_permit_while_the_last_one_is_dropped_then_the_idle_timer_is_not_left_armed()
 {
    let state = state_with_limit(10);
    let last = state.try_acquire().expect("permit");
    let (taken, receiver) = std::sync::mpsc::channel();
    let racer = Arc::clone(&state);
    state.before_next_arm(move || {
        let (finished, finish_signal) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let permit = racer.try_acquire();
            let _ = finished.send(());
            let _ = taken.send(permit);
        });
        let _ = finish_signal.recv_timeout(Duration::from_millis(300));
    });
    drop(last);
    let second = receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("the racing send ran")
        .expect("the racing send got a permit");
    assert!(!state.violates_idle_invariant());
    drop(second);
}

#[test]
fn given_no_settings_yet_then_the_limit_is_unknown_and_no_permit_is_given() {
    let state = bare_state();
    assert!(state.limit_unknown());
    assert!(state.try_acquire().is_none());
}

#[test]
fn given_a_limit_of_zero_then_the_connection_is_full_but_not_unknown_and_a_raise_opens_it() {
    let state = bare_state();
    state.apply_settings(Some(0));
    assert!(!state.limit_unknown());
    assert!(state.try_acquire().is_none());
    state.apply_settings(Some(2));
    assert!(state.try_acquire().is_some());
}

#[test]
fn given_a_first_settings_without_a_limit_then_the_streams_are_unbounded() {
    let state = bare_state();
    state.apply_settings(None);
    let permits: Vec<_> = (0..64).map(|_| state.try_acquire()).collect();
    assert!(permits.iter().all(Option::is_some));
}

#[test]
fn given_a_later_settings_without_a_limit_then_the_previous_limit_stays() {
    let state = state_with_limit(1);
    state.apply_settings(None);
    let first = state.try_acquire();
    assert!(first.is_some());
    assert!(state.try_acquire().is_none());
}
