use std::sync::Arc;

use tokio::time::{Instant, advance, pause};

use crate::acquire_tests::Rig;
use crate::dialer_rig_tests::h1_connection_watched;
use crate::h2state::H2State;
use crate::idle_cap_tests::{STEP, busy_connections, go_idle_one_step_apart};
use crate::limits::MAX_IDLE_PER_HOST;

fn retired_indices(states: &[Arc<H2State>]) -> Vec<usize> {
    states
        .iter()
        .enumerate()
        .filter(|(_, state)| state.is_retired())
        .map(|(index, _)| index)
        .collect()
}

fn h1_deadlines(rig: &Rig) -> Vec<Option<Instant>> {
    rig.shared
        .pool
        .lock()
        .get(&rig.key)
        .map(|host| {
            host.h1
                .iter()
                .map(|idle| idle.conn.idle.armed_until())
                .collect()
        })
        .expect("host")
}

#[tokio::test]
async fn given_nine_idle_http2_connections_when_the_first_dialed_goes_idle_last_then_the_one_idle_the_longest_is_closed()
 {
    let rig = Rig::silent_server().await;
    let (states, mut conns) = busy_connections(&rig, MAX_IDLE_PER_HOST + 1).await;
    pause();
    let first_dialed = conns.remove(0);
    go_idle_one_step_apart(conns).await;
    drop(first_dialed);
    assert_eq!(retired_indices(&states), vec![1]);
}

#[tokio::test]
async fn given_eight_idle_http2_connections_and_an_http1_connection_when_the_oldest_of_the_eight_is_not_the_first_dialed_then_it_is_the_one_closed()
 {
    let rig = Rig::silent_server().await;
    let (states, mut conns) = busy_connections(&rig, MAX_IDLE_PER_HOST).await;
    let (h1, _server) = rig.h1_connection().await;
    pause();
    let first_dialed = conns.remove(0);
    let mut rest = conns.into_iter();
    drop(rest.next());
    advance(STEP).await;
    rig.shared.pool.release_h1(&rig.key, h1);
    advance(STEP).await;
    go_idle_one_step_apart(rest.collect()).await;
    drop(first_dialed);
    assert_eq!(retired_indices(&states), vec![1]);
    assert_eq!(h1_deadlines(&rig).len(), 1);
}

#[tokio::test]
async fn given_nine_idle_http1_connections_one_of_them_expired_but_still_open_when_the_ninth_is_released_then_the_expired_one_is_closed()
 {
    let rig = Rig::offline();
    pause();
    let mut servers = Vec::new();
    for index in 0..=MAX_IDLE_PER_HOST {
        let (conn, server, watch) = h1_connection_watched().await;
        rig.shared.pool.release_h1(&rig.key, conn);
        if index == 3 {
            let at = h1_deadlines(&rig).get(3).copied().flatten().expect("armed");
            assert!(watch.expire(at));
        }
        servers.push((server, watch));
        advance(STEP).await;
    }
    let left = h1_deadlines(&rig);
    assert_eq!(left.len(), MAX_IDLE_PER_HOST);
    assert!(left.iter().all(Option::is_some));
}
