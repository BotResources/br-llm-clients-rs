use std::time::Duration;

use crate::acquire_tests::{Outcome, Rig};
use crate::dialer::Dialer;
use crate::h2track::track_streams;
use crate::host::Host;
use crate::limits::MAX_H2_PER_HOST;

const SOON: Duration = Duration::from_secs(5);
const SETTINGS_WAIT: Duration = Duration::from_secs(60);

async fn dial_that_outlived_the_last_http1_connection(rig: &Rig) -> Dialer {
    let (alive, dialer) = rig.ungated_dial_planned_while_an_http1_connection_is_alive();
    drop(alive);
    rig.shared.pool.forget_if_unused(&rig.key);
    dialer
}

async fn outcome_soon(waiter: tokio::task::JoinHandle<Outcome>) -> Outcome {
    tokio::time::timeout(SOON, waiter)
        .await
        .expect("the waiter never heard the connection it waits for: its wake went to a signal nobody listens to")
        .expect("join")
}

#[test]
fn given_a_host_with_an_ungated_dial_in_flight_then_it_is_in_use_until_the_dial_is_over() {
    let host = Host::new();
    let pin = host.pin_for_dial();
    assert!(host.in_use());
    drop(pin);
    assert!(!host.in_use());
}

#[tokio::test]
async fn given_an_ungated_dial_in_flight_when_the_last_http1_connection_closes_then_the_host_stays_in_the_pool()
 {
    let rig = Rig::silent_server().await;
    let _dialer = dial_that_outlived_the_last_http1_connection(&rig).await;
    assert!(rig.shared.pool.lock().contains_key(&rig.key));
}

#[tokio::test]
async fn given_an_ungated_dial_in_flight_when_another_host_is_planned_then_the_host_stays_in_the_pool()
 {
    let rig = Rig::silent_server().await;
    let (alive, _dialer) = rig.ungated_dial_planned_while_an_http1_connection_is_alive();
    drop(alive);
    let other = crate::pool::PoolKey::of(
        &crate::endpoint::Endpoint::https("https://other.example/").expect("endpoint"),
    );
    drop(rig.shared.pool.plan(&other, None));
    assert!(rig.shared.pool.lock().contains_key(&rig.key));
}

#[tokio::test]
async fn given_a_dial_that_started_on_an_http1_host_when_it_comes_back_as_http2_then_a_waiter_hears_the_first_settings()
 {
    let rig = Rig::silent_server().await;
    let dialer = dial_that_outlived_the_last_http1_connection(&rig).await;
    let (state, _conn) = rig.land(dialer).await;
    let waiter = rig.waiter();
    rig.waiting(1).await;
    state.apply_settings(Some(5));
    assert!(matches!(outcome_soon(waiter).await, Outcome::Ready));
}

#[tokio::test]
async fn given_a_dial_that_started_on_an_http1_host_when_it_comes_back_as_http2_then_a_waiter_hears_the_end_of_the_connection()
 {
    let rig = Rig::silent_server().await;
    let dialer = dial_that_outlived_the_last_http1_connection(&rig).await;
    let (state, _conn) = rig.land(dialer).await;
    let waiter = rig.waiter();
    rig.waiting(1).await;
    track_streams(async { Ok(()) }, state, SETTINGS_WAIT).await;
    assert!(matches!(outcome_soon(waiter).await, Outcome::Spent));
}

#[tokio::test]
async fn given_a_dial_that_started_on_an_http1_host_when_it_comes_back_as_http2_then_a_waiter_hears_a_stream_freed()
 {
    let rig = Rig::silent_server().await;
    let dialer = dial_that_outlived_the_last_http1_connection(&rig).await;
    let (first, first_conn) = rig.land(dialer).await;
    first.apply_settings(Some(1));
    let mut others = Vec::new();
    for _ in 1..MAX_H2_PER_HOST {
        let (state, conn) = rig.land(rig.dialer()).await;
        state.apply_settings(Some(1));
        others.push((state, conn));
    }
    let waiter = rig.waiter();
    rig.waiting(1).await;
    drop(first_conn);
    assert!(matches!(outcome_soon(waiter).await, Outcome::Ready));
}

#[tokio::test]
async fn given_a_connection_pooled_by_an_ungated_dial_then_it_wakes_the_signal_the_host_still_holds()
 {
    let rig = Rig::silent_server().await;
    let dialer = dial_that_outlived_the_last_http1_connection(&rig).await;
    let (state, _conn) = rig.land(dialer).await;
    let changes = rig
        .shared
        .pool
        .lock()
        .get(&rig.key)
        .map(|host| host.signal.subscribe())
        .expect("host");
    state.wake_host();
    assert!(changes.has_changed().expect("open"));
}
