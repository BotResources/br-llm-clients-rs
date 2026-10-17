use tokio::time::{advance, pause};

use crate::acquire_tests::{Outcome, Rig, outcome};
use crate::h2state_tests::state_with_limit;
use crate::idle_cap_tests::{STEP, busy_connections};
use crate::limits::MAX_IDLE_PER_HOST;
use crate::settled::Settled;

#[tokio::test]
async fn given_a_send_waiting_for_the_first_settings_of_the_connection_idle_the_longest_when_the_idle_cap_trims_it_then_the_send_loses_no_attempt()
 {
    let rig = Rig::silent_server().await;
    let (oldest, oldest_conn) = rig.land(rig.dialer()).await;
    let (_busy_states, busy) = busy_connections(&rig, MAX_IDLE_PER_HOST).await;
    let waiter = rig.waiter();
    rig.waiting(1).await;
    pause();
    drop(oldest_conn);
    advance(STEP).await;
    rig.waiting(1).await;
    drop(busy);
    oldest.announce_run(&[Some(1)]);
    assert!(oldest.is_retired(), "the idle cap must have trimmed it");
    assert!(matches!(outcome(waiter).await, Outcome::Ready));
}

#[tokio::test]
async fn given_a_send_waiting_for_a_dial_when_the_idle_cap_trims_the_connection_that_landed_and_was_idle_the_longest_then_the_send_loses_no_attempt()
 {
    let rig = Rig::silent_server().await;
    let dialer = rig.dialer();
    let waiter = rig.waiter();
    rig.waiting(1).await;
    let (oldest, oldest_conn) = rig.land(dialer).await;
    let (_busy_states, busy) = busy_connections(&rig, MAX_IDLE_PER_HOST).await;
    rig.waiting(1).await;
    pause();
    drop(oldest_conn);
    advance(STEP).await;
    rig.waiting(1).await;
    drop(busy);
    oldest.announce_run(&[Some(1)]);
    assert!(oldest.is_retired(), "the idle cap must have trimmed it");
    assert!(matches!(outcome(waiter).await, Outcome::Ready));
}

#[tokio::test]
async fn given_a_connection_that_ended_and_a_send_waiting_for_it_then_the_send_loses_an_attempt() {
    let rig = Rig::silent_server().await;
    let (state, _conn) = rig.land(rig.dialer()).await;
    let waiter = rig.waiter();
    rig.waiting(1).await;
    state.end();
    state.wake_host();
    assert!(matches!(outcome(waiter).await, Outcome::Spent));
}

#[tokio::test]
async fn given_eight_idle_connections_and_one_idle_connection_whose_settings_are_unread_when_they_are_read_then_the_oldest_idle_connection_is_trimmed()
 {
    let rig = Rig::silent_server().await;
    let (idle, idle_conns) = busy_connections(&rig, MAX_IDLE_PER_HOST).await;
    drop(idle_conns);
    let (late, late_conn) = rig.land(rig.ungated_dialer()).await;
    drop(late_conn);
    assert_eq!(rig.shared.pool.h2_connections(), MAX_IDLE_PER_HOST + 1);
    late.announce_run(&[Some(1)]);
    assert_eq!(rig.shared.pool.h2_connections(), MAX_IDLE_PER_HOST);
    assert!(idle[0].is_retired());
    assert!(!late.is_retired());
}

#[test]
fn given_a_trimmed_connection_then_a_send_that_claimed_it_is_not_told_the_attempt_is_spent() {
    let state = state_with_limit(5);
    state.trim();
    assert!(state.is_retired());
    assert!(state.try_acquire().is_none());
    assert!(state.settled().is_none());
}

#[test]
fn given_a_connection_that_ended_then_a_send_that_claimed_it_is_told_the_attempt_is_spent() {
    let state = state_with_limit(5);
    state.end();
    assert!(matches!(state.settled(), Some(Settled::Spent)));
}

#[test]
fn given_a_trimmed_connection_that_then_ends_then_a_send_that_claimed_it_still_loses_nothing() {
    let state = state_with_limit(5);
    state.trim();
    state.end();
    assert!(state.settled().is_none());
}
