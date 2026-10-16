use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tokio::sync::watch;

use crate::endpoint::Endpoint;
use crate::h2state::H2State;
use crate::idle::IdleClock;
use crate::pool::{HostHandle, Pool, PoolKey};
use crate::transport::Signal;

pub(crate) fn bare_state() -> Arc<H2State> {
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

pub(crate) fn signalled_state() -> (Arc<H2State>, Signal) {
    let pool = Arc::new(Pool::new(Duration::from_secs(30)));
    let key = PoolKey::of(&Endpoint::https("https://h.example/").expect("endpoint"));
    let signal: Signal = Arc::new(watch::channel(0u64).0);
    let host = HostHandle {
        signal: Arc::clone(&signal),
        pool: Arc::downgrade(&pool),
        key,
    };
    let (idle, _) = IdleClock::new(Duration::from_secs(30));
    (H2State::new(idle, host), signal)
}

pub(crate) fn pooled_state() -> (Arc<Pool>, Arc<H2State>) {
    let pool = Arc::new(Pool::new(Duration::from_secs(30)));
    let key = PoolKey::of(&Endpoint::https("https://h.example/").expect("endpoint"));
    let host = HostHandle {
        signal: Arc::new(watch::channel(0u64).0),
        pool: Arc::downgrade(&pool),
        key,
    };
    let (idle, _) = IdleClock::new(Duration::from_secs(30));
    (pool, H2State::new(idle, host))
}

pub(crate) fn state_with_limit(limit: u32) -> Arc<H2State> {
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

#[test]
fn given_an_idle_expiry_already_claimed_then_the_connection_gives_no_permit() {
    let pool = Arc::new(Pool::new(Duration::from_secs(30)));
    let key = PoolKey::of(&Endpoint::https("https://h.example/").expect("endpoint"));
    let host = HostHandle {
        signal: Arc::new(watch::channel(0u64).0),
        pool: Arc::downgrade(&pool),
        key,
    };
    let (idle, mut idle_watch) = IdleClock::new(Duration::from_secs(30));
    let state = H2State::new(idle, host);
    state.apply_settings(Some(10));
    state.arm_idle();
    let armed = idle_watch.armed_until().expect("armed");
    assert!(idle_watch.expire(armed));
    assert!(state.try_acquire().is_none());
    assert!(state.is_idle());
}

#[test]
fn given_an_idle_connection_that_announced_zero_then_it_is_never_trimmed_and_does_not_count_as_idle()
 {
    let state = state_with_limit(0);
    assert!(state.is_idle());
    assert!(!state.is_trimmable());
}

#[test]
fn given_an_idle_connection_with_room_then_it_can_be_trimmed() {
    let state = state_with_limit(2);
    assert!(state.is_trimmable());
    let permit = state.try_acquire().expect("permit");
    assert!(!state.is_trimmable());
    drop(permit);
}

#[test]
fn given_an_idle_connection_whose_settings_are_unknown_then_it_counts_as_full_and_is_not_trimmed() {
    let state = bare_state();
    assert!(state.is_idle());
    assert!(!state.is_trimmable());
    state.apply_settings(Some(2));
    assert!(state.is_trimmable());
}

#[test]
fn given_an_idle_connection_that_was_lowered_to_zero_after_usable_settings_then_it_is_not_trimmed()
{
    let state = state_with_limit(2);
    state.apply_settings(Some(0));
    assert!(state.is_idle());
    assert!(!state.is_trimmable());
}

#[test]
fn given_a_retired_connection_that_never_sent_its_settings_then_its_limit_is_still_unknown() {
    let state = bare_state();
    state.retire();
    assert!(state.limit_unknown());
}

#[test]
fn given_a_new_connection_whose_idle_expiry_was_already_claimed_then_the_dialer_gets_a_retired_connection()
 {
    let pool = Arc::new(Pool::new(Duration::from_secs(30)));
    let key = PoolKey::of(&Endpoint::https("https://h.example/").expect("endpoint"));
    let host = HostHandle {
        signal: Arc::new(watch::channel(0u64).0),
        pool: Arc::downgrade(&pool),
        key,
    };
    let (idle, mut idle_watch) = IdleClock::new(Duration::from_secs(30));
    let state = H2State::new(idle, host);
    state.arm_idle();
    let armed = idle_watch.armed_until().expect("armed");
    assert!(idle_watch.expire(armed));
    let permit = state.force_acquire();
    assert!(state.is_retired());
    drop(permit);
}

#[test]
fn given_a_new_connection_alive_then_the_dialer_gets_a_permit_and_it_is_not_retired() {
    let state = bare_state();
    state.arm_idle();
    let permit = state.force_acquire();
    assert!(!state.is_retired());
    drop(permit);
}

#[tokio::test]
async fn given_a_first_zero_then_the_permit_of_the_dialer_is_not_usable() {
    let state = bare_state();
    let permit = state.force_acquire();
    state.apply_settings(Some(0));
    assert!(!permit.usable_since_first_settings().await);
}

#[tokio::test]
async fn given_a_first_zero_raised_before_the_permit_asks_then_it_is_usable() {
    let state = bare_state();
    let permit = state.force_acquire();
    state.apply_settings(Some(0));
    state.apply_settings(Some(4));
    assert!(permit.usable_since_first_settings().await);
}

#[tokio::test]
async fn given_a_usable_first_settings_then_a_later_zero_leaves_the_permit_usable() {
    let state = bare_state();
    let permit = state.force_acquire();
    state.apply_settings_run(&[Some(4), Some(0)]);
    assert!(permit.usable_since_first_settings().await);
}

#[tokio::test]
async fn given_no_settings_yet_then_the_permit_waits_for_them_and_is_woken_by_the_first_ones() {
    let state = bare_state();
    let permit = state.force_acquire();
    let waiting = tokio::spawn({
        let state = Arc::clone(&state);
        async move {
            let permit = state.force_acquire();
            permit.usable_since_first_settings().await
        }
    });
    tokio::task::yield_now().await;
    assert!(!waiting.is_finished());
    state.apply_settings(Some(4));
    assert!(waiting.await.expect("join"));
    drop(permit);
}
