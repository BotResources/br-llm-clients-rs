use std::sync::Arc;

use crate::acquire_tests::Rig;
use crate::h2state::H2State;
use crate::limits::MAX_IDLE_PER_HOST;
use crate::transport::Conn;
use crate::turns::within_spins;

pub(crate) async fn land_with_limit(rig: &Rig, limit: u32) -> (Arc<H2State>, Conn) {
    let (state, conn) = rig.land(rig.dialer()).await;
    state.apply_settings(Some(limit));
    (state, conn)
}

#[tokio::test]
async fn given_connections_that_announced_zero_then_they_do_not_count_toward_the_idle_cap_and_are_never_trimmed()
 {
    let rig = Rig::silent_server().await;
    let mut zero = Vec::new();
    let mut open = Vec::new();
    for _ in 0..3 {
        zero.push(land_with_limit(&rig, 0).await);
    }
    for _ in 0..MAX_IDLE_PER_HOST + 2 {
        open.push(land_with_limit(&rig, 1).await);
    }
    let (zero_states, zero_conns): (Vec<_>, Vec<_>) = zero.into_iter().unzip();
    let (open_states, open_conns): (Vec<_>, Vec<_>) = open.into_iter().unzip();
    drop(zero_conns);
    drop(open_conns);
    assert!(zero_states.iter().all(|state| !state.is_retired()));
    let trimmed = open_states
        .iter()
        .filter(|state| state.is_retired())
        .count();
    assert_eq!(trimmed, 2);
    assert_eq!(rig.shared.pool.h2_connections(), 3 + MAX_IDLE_PER_HOST);
}

#[tokio::test]
async fn given_an_idle_connection_trimmed_to_keep_eight_then_the_sends_waiting_on_the_host_are_woken_after_it_is_retired()
 {
    let rig = Rig::silent_server().await;
    let mut states = Vec::new();
    let mut conns = Vec::new();
    for _ in 0..MAX_IDLE_PER_HOST + 1 {
        let (state, conn) = land_with_limit(&rig, 1).await;
        states.push(state);
        conns.push(conn);
    }
    let signal = rig
        .shared
        .pool
        .lock()
        .get(&rig.key)
        .map(|host| Arc::clone(&host.signal))
        .expect("host");
    let changes = signal.subscribe();
    let last = conns.pop().expect("a connection");
    drop(conns);
    assert!(states.iter().all(|state| !state.is_retired()));
    let before = *changes.borrow();
    let held_pool = rig.shared.pool.lock();
    let dropping = std::thread::spawn(move || drop(last));
    within_spins(
        || *changes.borrow() != before,
        || "the freed stream did not wake the host".to_owned(),
    );
    let after_the_freed_stream = *changes.borrow();
    assert!(!states.first().expect("oldest").is_retired());
    drop(held_pool);
    dropping.join().expect("the drop ends");
    assert!(states.first().expect("oldest").is_retired());
    assert_ne!(
        *changes.borrow(),
        after_the_freed_stream,
        "no wake follows the retirement"
    );
}
