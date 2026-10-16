use std::sync::Arc;
use std::sync::Weak;

use hyper::client::conn::http2;
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::io::DuplexStream;
use tokio::time::{Instant, advance, pause};

use crate::dialer_rig_tests::h1_connection_watched;
use crate::endpoint::Endpoint;
use crate::h2state::H2State;
use crate::host::{H2Entry, Host, Idle};
use crate::idle::IdleClock;
use crate::idle_cap_tests::STEP;
use crate::limits::MAX_IDLE_PER_HOST;
use crate::payload::Payload;
use crate::pool::{HostHandle, PoolKey};
use crate::tuning::Tuning;

pub(crate) async fn idle_entry(host: &Host, open: bool) -> (H2Entry, DuplexStream) {
    let (client, server) = tokio::io::duplex(4096);
    let (sender, connection) = http2::Builder::new(TokioExecutor::new())
        .handshake::<_, Payload>(TokioIo::new(client))
        .await
        .expect("handshake");
    if open {
        tokio::spawn(async move {
            let _ = connection.await;
        });
    } else {
        drop(connection);
    }
    assert_eq!(sender.is_closed(), !open);
    let key = PoolKey::of(&Endpoint::https("https://h.example/").expect("endpoint"));
    let handle = HostHandle {
        signal: Arc::clone(&host.signal),
        pool: Weak::new(),
        key,
    };
    let (idle, _watch) = IdleClock::new(Tuning::FIXED.idle_limit);
    let state = H2State::new(idle, handle);
    state.apply_settings(Some(1));
    state.arm_idle();
    (H2Entry { sender, state }, server)
}

async fn host_with_idle_entries(closed: &[usize], total: usize) -> (Host, Vec<DuplexStream>) {
    pause();
    let mut host = Host::new();
    let mut servers = Vec::new();
    for index in 0..total {
        let (entry, server) = idle_entry(&host, !closed.contains(&index)).await;
        host.h2.push(entry);
        servers.push(server);
        advance(STEP).await;
    }
    (host, servers)
}

async fn idle_h1(host: &mut Host) -> DuplexStream {
    let (conn, server, _watch) = h1_connection_watched().await;
    conn.idle.arm();
    host.h1.push(Idle {
        conn,
        since: Instant::now(),
    });
    server
}

fn states(host: &Host) -> Vec<Arc<H2State>> {
    host.h2
        .iter()
        .map(|entry| Arc::clone(&entry.state))
        .collect()
}

fn retired_indices(states: &[Arc<H2State>]) -> Vec<usize> {
    states
        .iter()
        .enumerate()
        .filter(|(_, state)| state.is_retired())
        .map(|(index, _)| index)
        .collect()
}

#[tokio::test]
async fn given_eight_idle_http2_connections_and_a_ninth_whose_sender_closed_but_was_not_swept_then_nothing_is_trimmed()
 {
    let (mut host, _servers) =
        host_with_idle_entries(&[MAX_IDLE_PER_HOST], MAX_IDLE_PER_HOST + 1).await;
    assert_eq!(host.idle_total(), MAX_IDLE_PER_HOST);
    let states = states(&host);
    assert!(!host.trim_to_cap());
    assert!(retired_indices(&states).is_empty());
    assert_eq!(host.h2.len(), MAX_IDLE_PER_HOST + 1);
}

#[tokio::test]
async fn given_nine_open_idle_http2_connections_and_a_closed_one_not_swept_between_them_then_only_the_open_one_idle_the_longest_is_trimmed()
 {
    let (mut host, _servers) = host_with_idle_entries(&[4], MAX_IDLE_PER_HOST + 2).await;
    assert_eq!(host.idle_total(), MAX_IDLE_PER_HOST + 1);
    let states = states(&host);
    assert!(host.trim_to_cap());
    assert_eq!(retired_indices(&states), vec![0]);
    assert_eq!(host.idle_total(), MAX_IDLE_PER_HOST);
}

#[tokio::test]
async fn given_idle_connections_with_the_same_deadline_when_trimmed_then_an_http1_connection_goes_before_any_http2_connection()
 {
    pause();
    let mut host = Host::new();
    let mut servers = Vec::new();
    for _ in 0..4 {
        servers.push(idle_h1(&mut host).await);
    }
    for _ in 0..MAX_IDLE_PER_HOST - 3 {
        let (entry, server) = idle_entry(&host, true).await;
        host.h2.push(entry);
        servers.push(server);
    }
    assert_eq!(host.idle_total(), MAX_IDLE_PER_HOST + 1);
    assert!(!host.trim_to_cap());
    assert_eq!(host.h1.len(), 3);
    assert_eq!(host.h2.len(), MAX_IDLE_PER_HOST - 3);
}

#[tokio::test]
async fn given_idle_http2_connections_with_the_same_deadline_when_trimmed_then_the_first_dialed_goes_first()
 {
    pause();
    let mut host = Host::new();
    let mut servers = Vec::new();
    for _ in 0..=MAX_IDLE_PER_HOST {
        let (entry, server) = idle_entry(&host, true).await;
        host.h2.push(entry);
        servers.push(server);
    }
    let states = states(&host);
    assert!(host.trim_to_cap());
    assert_eq!(retired_indices(&states), vec![0]);
}
