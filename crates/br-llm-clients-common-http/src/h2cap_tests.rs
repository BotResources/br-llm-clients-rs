use std::sync::Arc;

use crate::acquire_tests::{Outcome, Rig, SETTINGS_WAIT, outcome};
use crate::claim::Claim;
use crate::connect::{Opened, Target, open};
use crate::dialer::Dialer;
use crate::h2state::H2State;
use crate::host::Proto;
use crate::pool::{HostHandle, MAX_H2_PER_HOST};
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

fn handle(rig: &Rig) -> HostHandle {
    let hosts = rig.shared.pool.lock();
    let host = hosts.get(&rig.key).expect("host");
    HostHandle {
        signal: Arc::clone(&host.signal),
        pool: Arc::downgrade(&rig.shared.pool),
        key: rig.key.clone(),
    }
}

fn ungated(rig: &Rig) -> Dialer {
    Dialer::new(handle(rig), None)
}

fn gated(rig: &Rig) -> (Dialer, Claim) {
    let landing = {
        let mut hosts = rig.shared.pool.lock();
        let host = hosts.get_mut(&rig.key).expect("host");
        host.begin_gated_dial();
        Arc::clone(&host.landing)
    };
    (
        Dialer::new(handle(rig), Some(Arc::clone(&landing))),
        Claim::Landing(landing),
    )
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
    assert!(dialer.register(opened).is_none());
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
    assert!(dialer.register(opened).is_some());
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
    assert!(dialer.register(opened).is_none());
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
