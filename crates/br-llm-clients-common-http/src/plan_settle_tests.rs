use std::sync::Arc;
use std::time::Duration;

use hyper::client::conn::http2;
use hyper_util::rt::{TokioExecutor, TokioIo};

use crate::acquire_tests::Rig;
use crate::claim::Claim;
use crate::endpoint::Endpoint;
use crate::h2state::H2State;
use crate::host::{H2Entry, Host};
use crate::idle::IdleClock;
use crate::payload::Payload;
use crate::pool::{HostHandle, Plan, Pool, PoolKey};
use crate::standing::Settled;
use crate::transport::H2Sender;

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
    let state = {
        let mut hosts = pool.lock();
        let host = hosts.entry(key.clone()).or_insert_with(Host::new);
        let handle = HostHandle {
            signal: Arc::clone(&host.signal),
            pool: Arc::downgrade(&pool),
            key: key.clone(),
        };
        let (idle, _) = IdleClock::new(Duration::from_secs(30));
        let state = H2State::new(idle, handle);
        state.apply_settings(Some(5));
        host.h2.push(H2Entry {
            sender,
            state: Arc::clone(&state),
        });
        state
    };
    assert!(!state.is_retired());
    Ended { pool, key, state }
}

#[tokio::test]
async fn given_a_connection_whose_sender_closed_before_it_was_retired_when_a_waiter_for_it_plans_then_the_waiter_spends_the_attempt()
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
async fn given_a_connection_whose_sender_closed_before_it_was_retired_when_a_send_plans_then_the_connection_leaves_the_pool_retired()
 {
    let ended = pool_holding_a_closed_connection_that_is_not_retired().await;
    assert!(matches!(ended.pool.plan(&ended.key, None), Plan::Dial(_)));
    assert_eq!(ended.pool.h2_connections(), 0);
    assert!(ended.state.is_retired());
}

#[tokio::test]
async fn given_a_connection_whose_sender_closed_before_it_was_retired_when_the_host_is_swept_then_the_connection_leaves_the_pool_retired()
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
