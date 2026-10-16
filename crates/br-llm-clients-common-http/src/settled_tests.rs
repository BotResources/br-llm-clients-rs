use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::h2state::H2State;
use crate::h2state_tests::{bare_state, pooled_state, state_with_limit};
use crate::hot_readers::while_looked_at;
use crate::settled::Settled;

fn is_open(state: &H2State) -> bool {
    state.settled().is_none()
}

fn is_spent(state: &H2State) -> bool {
    matches!(state.settled(), Some(Settled::Spent))
}

#[test]
fn given_a_connection_that_is_alive_then_a_send_that_waited_for_it_has_lost_nothing() {
    assert!(is_open(&bare_state()));
    assert!(is_open(&state_with_limit(5)));
}

#[test]
fn given_a_connection_that_was_retired_after_its_settings_then_a_send_that_waited_for_it_has_lost_an_attempt()
 {
    let state = state_with_limit(5);
    state.end();
    assert!(is_spent(&state));
}

#[test]
fn given_a_connection_that_never_sent_its_settings_then_a_send_that_waited_for_it_has_lost_an_attempt()
 {
    let state = bare_state();
    state.end();
    assert!(state.limit_unknown());
    assert!(is_spent(&state));
}

#[test]
fn given_a_first_settings_with_a_limit_of_zero_then_a_send_that_waited_for_it_has_lost_an_attempt()
{
    let state = bare_state();
    state.apply_settings(Some(0));
    assert!(is_spent(&state));
}

#[test]
fn given_a_limit_of_zero_in_the_first_settings_and_a_raise_later_then_the_connection_is_usable_for_the_sends_that_start_after_it()
 {
    let state = bare_state();
    state.apply_settings(Some(0));
    assert!(is_spent(&state));
    state.apply_settings(Some(3));
    assert!(is_open(&state));
    assert!(state.try_acquire().is_some());
}

#[test]
fn given_a_raise_and_then_a_new_zero_after_a_first_zero_then_the_connection_stays_usable() {
    let state = bare_state();
    state.apply_settings_run(&[Some(0), Some(3), Some(0)]);
    assert!(is_open(&state));
}

#[test]
fn given_a_first_zero_and_a_raise_in_the_same_run_then_the_connection_is_open_and_gives_a_permit() {
    let state = bare_state();
    state.apply_settings_run(&[Some(0), Some(100)]);
    assert!(is_open(&state));
    assert!(state.try_acquire().is_some());
}

#[test]
fn given_a_usable_first_settings_and_a_zero_in_the_same_run_then_the_connection_is_usable_and_full()
{
    let state = bare_state();
    state.apply_settings_run(&[Some(100), Some(0)]);
    assert!(is_open(&state));
    assert!(state.try_acquire().is_none());
}

#[test]
fn given_a_first_settings_without_a_limit_and_a_zero_in_the_same_run_then_the_connection_is_usable()
{
    let state = bare_state();
    state.apply_settings_run(&[None, Some(0)]);
    assert!(is_open(&state));
}

#[test]
fn given_a_limit_lowered_to_zero_after_a_usable_first_settings_then_the_send_that_waited_has_lost_nothing()
 {
    let state = state_with_limit(5);
    state.apply_settings(Some(0));
    assert!(is_open(&state));
}

#[test]
fn given_a_violation_then_it_is_reported_with_its_text_and_not_as_a_lost_attempt() {
    let state = bare_state();
    state.record_violation("the server broke a rule".to_owned());
    assert!(matches!(
        state.settled(),
        Some(Settled::Broken(text)) if text == "the server broke a rule"
    ));
}

fn fresh_states(count: usize) -> Vec<Arc<H2State>> {
    (0..count).map(|_| bare_state()).collect()
}

#[test]
fn given_first_runs_of_a_zero_and_a_raise_applied_while_other_threads_look_then_no_look_sees_the_zero()
 {
    let states = fresh_states(5_000);
    let zeros_seen = AtomicUsize::new(0);
    let looks = while_looked_at(
        &states,
        2,
        |state| {
            if is_spent(state) {
                zeros_seen.fetch_add(1, Ordering::Relaxed);
            }
            drop(state.try_acquire());
        },
        |state| state.apply_settings_run(&[Some(0), Some(100)]),
    );
    assert!(looks > 0);
    assert_eq!(zeros_seen.load(Ordering::Relaxed), 0);
    assert!(states.iter().all(|state| is_open(state)));
}

#[test]
fn given_first_zeros_applied_while_other_threads_look_then_a_look_that_sees_the_settings_known_sees_the_zero()
 {
    let states = fresh_states(5_000);
    let known_without_the_zero = AtomicUsize::new(0);
    while_looked_at(
        &states,
        2,
        |state| {
            if !state.awaiting_settings() && !is_spent(state) {
                known_without_the_zero.fetch_add(1, Ordering::Relaxed);
            }
        },
        |state| state.apply_settings_run(&[Some(0)]),
    );
    assert_eq!(known_without_the_zero.load(Ordering::Relaxed), 0);
}

#[test]
fn given_a_violation_recorded_and_the_connection_retired_between_the_two_reads_of_a_look_then_the_look_reports_the_violation()
 {
    let (pool, state) = pooled_state();
    state.apply_settings(Some(5));
    pool.hooks.on_settled_step({
        let state = Arc::clone(&state);
        move || {
            state.record_violation("the server broke a rule".to_owned());
            state.end();
        }
    });
    assert!(matches!(
        state.settled(),
        Some(Settled::Broken(text)) if text == "the server broke a rule"
    ));
}

#[test]
fn given_a_violation_then_a_retirement_while_other_threads_look_then_no_look_reports_a_spent_attempt()
 {
    let states: Vec<_> = (0..2_000).map(|_| state_with_limit(5)).collect();
    let spent = AtomicUsize::new(0);
    let looks = while_looked_at(
        &states,
        8,
        |state| {
            if is_spent(state) {
                spent.fetch_add(1, Ordering::Relaxed);
            }
        },
        |state| {
            state.record_violation("the server broke a rule".to_owned());
            state.end();
        },
    );
    assert!(looks > 0);
    assert_eq!(spent.load(Ordering::Relaxed), 0);
}
