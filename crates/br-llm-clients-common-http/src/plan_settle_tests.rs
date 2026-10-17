use std::sync::Arc;
use std::time::Duration;

use hyper::client::conn::{http1, http2};
use hyper_util::rt::{TokioExecutor, TokioIo};

use crate::acquire_tests::Rig;
use crate::claim::{Claim, Landing};
use crate::dialer::DialFailure;
use crate::endpoint::Endpoint;
use crate::h2state::H2State;
use crate::host::{H2Entry, Host};
use crate::idle::IdleClock;
use crate::payload::Payload;
use crate::pool::{HostHandle, Plan, Pool, PoolKey, WaitKind};
use crate::settled::Settled;
use crate::transport::{H1Conn, H2Sender};

async fn closed_sender() -> H2Sender {
    let (client_io, _server_io) = tokio::io::duplex(65536);
    let (sender, connection) = http2::Builder::new(TokioExecutor::new())
        .handshake::<_, Payload>(TokioIo::new(client_io))
        .await
        .expect("handshake");
    drop(connection);
    assert!(sender.is_closed());
    sender
}

struct Ended {
    pool: Arc<Pool>,
    key: PoolKey,
    state: Arc<H2State>,
}

async fn pool_holding_a_closed_connection_that_is_not_retired() -> Ended {
    let sender = closed_sender().await;
    let pool = Arc::new(Pool::new(Duration::from_secs(30)));
    let key = PoolKey::of(&Endpoint::https("https://h.example/").expect("endpoint"));
    let handle = {
        let mut hosts = pool.lock();
        let host = hosts.entry(key.clone()).or_insert_with(Host::new);
        HostHandle {
            signal: Arc::clone(&host.signal),
            pool: Arc::downgrade(&pool),
            key: key.clone(),
        }
    };
    let (idle, _) = IdleClock::new(Duration::from_secs(30));
    let state = H2State::new(idle, handle);
    state.apply_settings(Some(5));
    pool.lock().get_mut(&key).expect("host").h2.push(H2Entry {
        sender,
        state: Arc::clone(&state),
    });
    assert!(!state.is_retired());
    Ended { pool, key, state }
}

#[tokio::test]
async fn defence_in_depth_given_a_connection_whose_sender_closed_before_it_was_retired_when_a_waiter_for_it_plans_then_the_waiter_spends_the_attempt()
 {
    let ended = pool_holding_a_closed_connection_that_is_not_retired().await;
    let claim = Claim::Conn(Arc::clone(&ended.state));
    assert!(matches!(
        ended.pool.plan(&ended.key, Some(&claim)),
        Plan::Settled(Settled::Spent)
    ));
    assert_eq!(ended.pool.h2_connections(), 0);
}

#[tokio::test]
async fn defence_in_depth_given_a_connection_whose_sender_closed_before_it_was_retired_when_a_send_plans_then_the_connection_leaves_the_pool_retired()
 {
    let ended = pool_holding_a_closed_connection_that_is_not_retired().await;
    assert!(matches!(ended.pool.plan(&ended.key, None), Plan::Dial(_)));
    assert_eq!(ended.pool.h2_connections(), 0);
    assert!(ended.state.is_retired());
}

#[tokio::test]
async fn defence_in_depth_given_a_connection_whose_sender_closed_before_it_was_retired_when_the_host_is_swept_then_the_connection_leaves_the_pool_retired()
 {
    let ended = pool_holding_a_closed_connection_that_is_not_retired().await;
    ended.pool.forget_if_unused(&ended.key);
    assert_eq!(ended.pool.h2_connections(), 0);
    assert!(ended.state.is_retired());
}

#[tokio::test]
async fn given_a_connection_whose_first_settings_announce_zero_during_the_plan_of_a_waiter_for_it_then_the_waiter_spends_the_attempt_and_dials_nothing()
 {
    let rig = Rig::silent_server().await;
    let (state, _conn) = rig.land(rig.dialer()).await;
    let claim = Claim::Conn(Arc::clone(&state));
    rig.shared.pool.hooks.on_before_scan({
        let state = Arc::clone(&state);
        move || state.apply_settings(Some(0))
    });
    assert!(matches!(
        rig.shared.pool.plan(&rig.key, Some(&claim)),
        Plan::Settled(Settled::Spent)
    ));
}

fn failed_dial_claim() -> Claim {
    let landing = Landing::new();
    landing.fail(DialFailure::Connect("refused".to_owned()));
    Claim::Landing(landing)
}

#[tokio::test]
async fn given_a_failed_dial_and_a_connection_with_a_free_stream_when_its_waiter_plans_then_the_failure_wins_over_the_stream()
 {
    let rig = Rig::silent_server().await;
    let (state, _held) = rig.land(rig.dialer()).await;
    state.apply_settings(Some(5));
    let claim = failed_dial_claim();
    assert!(matches!(
        rig.shared.pool.plan(&rig.key, Some(&claim)),
        Plan::Settled(Settled::Failed(_))
    ));
}

#[tokio::test]
async fn given_a_failed_dial_and_an_idle_http1_connection_when_its_waiter_plans_then_the_failure_wins_over_the_connection()
 {
    let rig = Rig::silent_server().await;
    let (client_io, _server_io) = tokio::io::duplex(65536);
    let (sender, connection) = http1::Builder::new()
        .handshake::<_, Payload>(TokioIo::new(client_io))
        .await
        .expect("handshake");
    let driver = tokio::spawn(connection);
    let (idle, _) = IdleClock::new(Duration::from_secs(30));
    rig.shared
        .pool
        .release_h1(&rig.key, H1Conn::new(sender, idle, driver.abort_handle()));
    let claim = failed_dial_claim();
    assert!(matches!(
        rig.shared.pool.plan(&rig.key, Some(&claim)),
        Plan::Settled(Settled::Failed(_))
    ));
    assert!(matches!(
        rig.shared.pool.plan(&rig.key, None),
        Plan::TryIdle(_)
    ));
}

#[tokio::test]
async fn given_a_connection_whose_first_settings_announce_zero_right_after_the_scan_for_unannounced_connections_when_a_send_plans_then_the_wake_is_already_in_its_wait()
 {
    let rig = Rig::silent_server().await;
    let (state, _held) = rig.land(rig.dialer()).await;
    rig.shared.pool.hooks.on_after_unannounced({
        let state = Arc::clone(&state);
        move || state.apply_settings(Some(0))
    });
    let Plan::Wait(wait) = rig.shared.pool.plan(&rig.key, None) else {
        panic!("the send waits for the first settings it saw missing");
    };
    assert!(matches!(
        &wait.kind,
        WaitKind::Stream(Some(Claim::Conn(waited))) if Arc::ptr_eq(waited, &state)
    ));
    assert!(wait.changed.has_changed().expect("open"));
}

#[tokio::test]
async fn given_a_connection_whose_first_settings_announce_zero_right_after_the_scan_for_unannounced_connections_when_a_waiter_for_it_plans_then_it_spends_the_attempt()
 {
    let rig = Rig::silent_server().await;
    let (state, _held) = rig.land(rig.dialer()).await;
    let claim = Claim::Conn(Arc::clone(&state));
    rig.shared.pool.hooks.on_after_unannounced({
        let state = Arc::clone(&state);
        move || state.apply_settings(Some(0))
    });
    assert!(matches!(
        rig.shared.pool.plan(&rig.key, Some(&claim)),
        Plan::Settled(Settled::Spent)
    ));
    rig.shared.pool.hooks.on_after_unannounced(|| {});
    assert!(matches!(
        rig.shared.pool.plan(&rig.key, None),
        Plan::Dial(_)
    ));
}
