use std::sync::Arc;

use crate::acquire_tests::{Outcome, Rig, SETTINGS_WAIT, outcome};
use crate::claim::Claim;
use crate::connect::{Opened, Target, open};
use crate::dialer::{Dialer, Registered};
use crate::h2state::H2State;
use crate::host::Proto;
use crate::limits::MAX_H2_PER_HOST;
use crate::settled::Settled;
use crate::transport::Conn;

async fn pooled_full_connections(rig: &Rig, count: usize) -> Vec<(Arc<H2State>, Conn)> {
    let mut kept = Vec::new();
    for _ in 0..count {
        let (state, conn) = rig.land(rig.dialer()).await;
        state.apply_settings(Some(1));
        kept.push((state, conn));
    }
    kept
}

fn ungated(rig: &Rig) -> Dialer {
    rig.ungated_dialer()
}

fn gated(rig: &Rig) -> (Dialer, Claim) {
    rig.gated_dialer()
}

async fn opened(rig: &Rig, dialer: &Dialer) -> Opened {
    let Ok(opened) = open(
        &rig.shared.tls,
        &Target::of(&rig.endpoint),
        &dialer.host,
        rig.shared.tuning,
        SETTINGS_WAIT,
    )
    .await
    else {
        panic!("the connection opens");
    };
    assert!(matches!(opened, Opened::H2 { .. }));
    opened
}

fn state_of(opened: &Opened) -> Arc<H2State> {
    match opened {
        Opened::H2 { state, .. } => Arc::clone(state),
        Opened::H1(_) => panic!("the connection is HTTP/2"),
    }
}

#[tokio::test]
async fn given_sixteen_pooled_http2_connections_when_an_ungated_dial_opens_another_then_it_is_retired_and_not_pooled()
 {
    let rig = Rig::silent_server().await;
    let _kept = pooled_full_connections(&rig, MAX_H2_PER_HOST).await;
    let dialer = ungated(&rig);
    let opened = opened(&rig, &dialer).await;
    let state = state_of(&opened);
    assert!(matches!(dialer.register(opened), Registered::Rejected));
    assert_eq!(rig.shared.pool.h2_connections(), MAX_H2_PER_HOST);
    assert!(state.is_retired());
}

#[tokio::test]
async fn given_fifteen_pooled_http2_connections_when_an_ungated_dial_opens_another_then_it_is_pooled_and_used()
 {
    let rig = Rig::silent_server().await;
    let _kept = pooled_full_connections(&rig, MAX_H2_PER_HOST - 1).await;
    let dialer = ungated(&rig);
    let opened = opened(&rig, &dialer).await;
    let state = state_of(&opened);
    assert!(matches!(dialer.register(opened), Registered::Connected(_)));
    assert_eq!(rig.shared.pool.h2_connections(), MAX_H2_PER_HOST);
    assert!(!state.is_retired());
}

#[tokio::test]
async fn given_a_gated_dial_that_meets_sixteen_pooled_connections_then_the_send_waiting_for_it_spends_one_attempt_and_the_gate_opens()
 {
    let rig = Rig::silent_server().await;
    let _kept = pooled_full_connections(&rig, MAX_H2_PER_HOST - 1).await;
    let (dialer, waiter) = gated(&rig);
    let opened = opened(&rig, &dialer).await;
    let (filler, _filler_conn) = rig.land(ungated(&rig)).await;
    filler.apply_settings(Some(1));
    assert!(waiter.settled().is_none());
    assert!(matches!(dialer.register(opened), Registered::Rejected));
    assert!(matches!(waiter.settled(), Some(Settled::Spent)));
    assert_eq!(rig.shared.pool.h2_connections(), MAX_H2_PER_HOST);
    assert!(!rig.dialing());
}

#[tokio::test]
async fn given_sixteen_pooled_connections_and_an_ungated_dial_that_opens_http2_when_a_send_makes_it_then_the_send_spends_one_attempt_and_dials_no_more()
 {
    let rig = Rig::silent_server().await;
    let _kept = pooled_full_connections(&rig, MAX_H2_PER_HOST).await;
    rig.shared
        .pool
        .lock()
        .get_mut(&rig.key)
        .expect("host")
        .proto = Some(Proto::H1);
    assert!(matches!(outcome(rig.waiter()).await, Outcome::Spent));
    assert_eq!(rig.shared.pool.h2_connections(), MAX_H2_PER_HOST);
    assert_eq!(rig.accepted(), MAX_H2_PER_HOST + 1);
}

#[tokio::test]
async fn given_sixteen_pooled_connections_of_which_one_is_retired_when_an_ungated_dial_opens_another_then_it_is_pooled_and_used()
 {
    let rig = Rig::silent_server().await;
    let kept = pooled_full_connections(&rig, MAX_H2_PER_HOST).await;
    kept[0].0.end();
    let dialer = ungated(&rig);
    let opened = opened(&rig, &dialer).await;
    let state = state_of(&opened);
    assert!(matches!(dialer.register(opened), Registered::Connected(_)));
    assert!(!state.is_retired());
    assert_eq!(rig.shared.pool.h2_connections(), MAX_H2_PER_HOST);
}

#[tokio::test]
async fn given_sixteen_pooled_connections_of_which_one_is_retired_when_a_gated_dial_opens_another_then_the_send_waiting_for_it_loses_nothing()
 {
    let rig = Rig::silent_server().await;
    let kept = pooled_full_connections(&rig, MAX_H2_PER_HOST - 1).await;
    let (dialer, waiter) = gated(&rig);
    let opened = opened(&rig, &dialer).await;
    let (filler, _filler_conn) = rig.land(ungated(&rig)).await;
    filler.apply_settings(Some(1));
    kept[0].0.end();
    assert!(matches!(dialer.register(opened), Registered::Connected(_)));
    assert!(waiter.settled().is_none());
    assert!(!rig.dialing());
}

#[tokio::test]
async fn given_a_host_known_as_http1_when_an_http2_connection_is_rejected_then_the_host_is_known_as_http2()
 {
    let rig = Rig::silent_server().await;
    let _kept = pooled_full_connections(&rig, MAX_H2_PER_HOST).await;
    let dialer = ungated(&rig);
    let opened = opened(&rig, &dialer).await;
    rig.shared
        .pool
        .lock()
        .get_mut(&rig.key)
        .expect("host")
        .proto = Some(Proto::H1);
    assert!(matches!(dialer.register(opened), Registered::Rejected));
    let hosts = rig.shared.pool.lock();
    assert!(hosts.get(&rig.key).expect("host").proto == Some(Proto::H2));
}

#[tokio::test]
async fn given_sixteen_pooled_connections_when_an_http2_connection_is_rejected_then_the_waiters_of_the_host_are_woken_by_the_registration_itself()
 {
    let rig = Rig::silent_server().await;
    let _kept = pooled_full_connections(&rig, MAX_H2_PER_HOST).await;
    let dialer = ungated(&rig);
    let opened = opened(&rig, &dialer).await;
    let changes = rig
        .shared
        .pool
        .lock()
        .get(&rig.key)
        .expect("host")
        .signal
        .subscribe();
    assert!(matches!(dialer.register(opened), Registered::Rejected));
    assert!(changes.has_changed().expect("open"));
}

#[tokio::test]
async fn given_a_host_that_left_the_pool_when_its_dial_returns_then_the_connection_is_not_pooled_and_no_host_is_made()
 {
    let rig = Rig::silent_server().await;
    let dialer = ungated(&rig);
    let opened = opened(&rig, &dialer).await;
    rig.shared.pool.lock().clear();
    assert!(matches!(dialer.register(opened), Registered::PoolGone));
    assert!(rig.shared.pool.lock().is_empty());
}
