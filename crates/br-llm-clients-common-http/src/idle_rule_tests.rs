use std::sync::Arc;
use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::time::{advance, pause, timeout};

use crate::acquire_tests::Rig;
use crate::h2state::H2State;
use crate::host::Idle;
use crate::idle_cap_tests::land_with_limit;
use crate::limits::MAX_IDLE_PER_HOST;
use crate::transport::{Conn, Signal};
use crate::turns::within_turns;

const STEP: Duration = Duration::from_millis(1);

async fn busy_connections(rig: &Rig, count: usize) -> (Vec<Arc<H2State>>, Vec<Conn>) {
    let mut states = Vec::new();
    let mut conns = Vec::new();
    for _ in 0..count {
        let (state, conn) = land_with_limit(rig, 1).await;
        states.push(state);
        conns.push(conn);
    }
    (states, conns)
}

fn idle_total(rig: &Rig) -> usize {
    rig.shared
        .pool
        .lock()
        .get(&rig.key)
        .map(|host| host.idle_total())
        .expect("host")
}

fn host_signal(rig: &Rig) -> Signal {
    rig.shared
        .pool
        .lock()
        .get(&rig.key)
        .map(|host| Arc::clone(&host.signal))
        .expect("host")
}

fn live(states: &[Arc<H2State>]) -> usize {
    states.iter().filter(|state| !state.is_retired()).count()
}

async fn go_idle_one_step_apart(conns: Vec<Conn>) {
    for conn in conns {
        drop(conn);
        advance(STEP).await;
    }
}

#[tokio::test]
async fn given_eight_idle_http2_connections_when_an_http1_connection_is_released_then_the_oldest_idle_connection_is_closed_and_eight_stay()
 {
    let rig = Rig::silent_server().await;
    let (states, conns) = busy_connections(&rig, MAX_IDLE_PER_HOST).await;
    let (h1, _server) = rig.h1_connection().await;
    pause();
    go_idle_one_step_apart(conns).await;
    assert_eq!(idle_total(&rig), MAX_IDLE_PER_HOST);
    let changes = host_signal(&rig).subscribe();
    let before = *changes.borrow();
    rig.shared.pool.release_h1(&rig.key, h1);
    assert_eq!(idle_total(&rig), MAX_IDLE_PER_HOST);
    assert_ne!(*changes.borrow(), before, "no wake follows the trimming");
    assert!(states[0].is_retired());
    assert_eq!(live(&states), MAX_IDLE_PER_HOST - 1);
    let hosts = rig.shared.pool.lock();
    assert_eq!(hosts.get(&rig.key).expect("host").h1.len(), 1);
}

#[tokio::test]
async fn given_an_idle_http1_connection_older_than_eight_http2_connections_when_the_ninth_goes_idle_then_the_http1_connection_is_closed()
 {
    let rig = Rig::silent_server().await;
    let (states, conns) = busy_connections(&rig, MAX_IDLE_PER_HOST).await;
    let (h1, _server) = rig.h1_connection().await;
    pause();
    rig.shared.pool.release_h1(&rig.key, h1);
    advance(STEP).await;
    go_idle_one_step_apart(conns).await;
    assert_eq!(idle_total(&rig), MAX_IDLE_PER_HOST);
    assert_eq!(live(&states), MAX_IDLE_PER_HOST);
    let hosts = rig.shared.pool.lock();
    assert!(hosts.get(&rig.key).expect("host").h1.is_empty());
}

#[tokio::test]
async fn given_eight_idle_http1_connections_when_another_is_released_then_the_oldest_is_closed_and_eight_stay()
 {
    let rig = Rig::silent_server().await;
    let mut servers = Vec::new();
    for _ in 0..=MAX_IDLE_PER_HOST {
        let (h1, server) = rig.h1_connection().await;
        rig.shared.pool.release_h1(&rig.key, h1);
        servers.push(server);
    }
    assert_eq!(idle_total(&rig), MAX_IDLE_PER_HOST);
    let mut oldest = servers.remove(0);
    let mut byte = [0u8; 1];
    let closed = timeout(Duration::from_secs(30), oldest.read(&mut byte))
        .await
        .expect("the oldest connection is closed");
    assert!(matches!(closed, Ok(0)));
    let newest = servers.last_mut().expect("newest");
    assert!(
        timeout(Duration::ZERO, newest.read(&mut byte))
            .await
            .is_err()
    );
}

async fn nine_idle_connections_one_of_them_retired_but_not_swept(retired: usize) {
    let rig = Rig::silent_server().await;
    let (states, mut conns) = busy_connections(&rig, MAX_IDLE_PER_HOST + 1).await;
    states[retired].end();
    drop(conns.remove(retired));
    drop(conns);
    assert_eq!(live(&states), MAX_IDLE_PER_HOST);
    assert_eq!(idle_total(&rig), MAX_IDLE_PER_HOST);
}

#[tokio::test]
async fn given_the_oldest_of_nine_idle_connections_retired_but_not_swept_then_it_does_not_count_and_no_live_one_is_trimmed()
 {
    nine_idle_connections_one_of_them_retired_but_not_swept(0).await;
}

#[tokio::test]
async fn given_a_younger_one_of_nine_idle_connections_retired_but_not_swept_then_it_does_not_count_and_no_live_one_is_trimmed()
 {
    nine_idle_connections_one_of_them_retired_but_not_swept(MAX_IDLE_PER_HOST / 2).await;
}

#[tokio::test]
async fn given_a_closed_http1_connection_not_yet_swept_and_eight_idle_http2_connections_then_it_does_not_count_and_no_live_one_is_trimmed()
 {
    let rig = Rig::silent_server().await;
    let (states, mut conns) = busy_connections(&rig, MAX_IDLE_PER_HOST).await;
    let (h1, server) = rig.h1_connection().await;
    drop(server);
    within_turns(
        || h1.sender.is_closed(),
        || "the closed HTTP/1 connection never showed as closed".to_owned(),
    )
    .await;
    pause();
    let last = conns.pop().expect("a connection");
    go_idle_one_step_apart(conns).await;
    h1.idle.arm();
    rig.shared
        .pool
        .lock()
        .get_mut(&rig.key)
        .expect("host")
        .h1
        .push(Idle {
            conn: h1,
            since: tokio::time::Instant::now(),
        });
    advance(STEP).await;
    drop(last);
    assert_eq!(live(&states), MAX_IDLE_PER_HOST);
    assert_eq!(idle_total(&rig), MAX_IDLE_PER_HOST);
}

#[tokio::test]
async fn given_ten_idle_connections_that_all_count_when_the_pool_trims_then_the_two_oldest_are_closed()
 {
    let rig = Rig::silent_server().await;
    let mut states = Vec::new();
    let mut conns = Vec::new();
    for _ in 0..MAX_IDLE_PER_HOST + 2 {
        let (state, conn) = rig.land(rig.ungated_dialer()).await;
        states.push(state);
        conns.push(conn);
    }
    drop(conns);
    for state in &states {
        state.apply_settings(Some(1));
    }
    assert_eq!(idle_total(&rig), MAX_IDLE_PER_HOST + 2);
    rig.shared.pool.trim_idle(&rig.key);
    assert_eq!(idle_total(&rig), MAX_IDLE_PER_HOST);
    assert!(states[0].is_retired() && states[1].is_retired());
    assert_eq!(live(&states), MAX_IDLE_PER_HOST);
}
