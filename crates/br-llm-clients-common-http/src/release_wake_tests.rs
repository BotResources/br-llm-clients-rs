use tokio::time::pause;

use crate::acquire_tests::{Outcome, Rig, outcome};
use crate::host::Host;
use crate::idle_cap_tests::{busy_connections, host_signal};
use crate::limits::MAX_H2_PER_HOST;

#[tokio::test]
async fn given_an_http1_connection_put_back_in_a_host_under_the_idle_cap_then_the_host_is_woken() {
    let rig = Rig::silent_server().await;
    let (h1, _server) = rig.h1_connection().await;
    rig.shared
        .pool
        .lock()
        .entry(rig.key.clone())
        .or_insert_with(Host::new);
    let changes = host_signal(&rig).subscribe();
    let before = *changes.borrow();
    rig.shared.pool.release_h1(&rig.key, h1);
    assert_ne!(*changes.borrow(), before);
}

#[tokio::test]
async fn given_a_send_waiting_for_a_stream_on_a_full_http2_host_when_an_http1_connection_is_put_back_then_the_send_takes_it_without_waiting_for_the_headers_timeout()
 {
    let rig = Rig::silent_server().await;
    let (_states, conns) = busy_connections(&rig, MAX_H2_PER_HOST).await;
    let (h1, _server) = rig.h1_connection().await;
    let waiter = rig.waiter();
    rig.waiting(1).await;
    pause();
    rig.shared.pool.release_h1(&rig.key, h1);
    assert!(matches!(outcome(waiter).await, Outcome::Ready));
    let hosts = rig.shared.pool.lock();
    assert!(hosts.get(&rig.key).expect("host").h1.is_empty());
    drop(hosts);
    drop(conns);
}
