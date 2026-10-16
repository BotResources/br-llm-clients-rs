use std::sync::Arc;
use std::time::Duration;

use crate::acquire_tests::Rig;
use crate::endpoint::Endpoint;
use crate::host::Host;
use crate::pool::{Plan, Pool, PoolKey, WaitKind};
use crate::transport::LiveGuard;

fn key(host: &str) -> PoolKey {
    PoolKey::of(&Endpoint::https(&format!("https://{host}/")).expect("endpoint"))
}

#[test]
fn given_a_host_with_a_dial_in_flight_then_the_sweep_does_not_forget_it() {
    let pool = Arc::new(Pool::new(Duration::from_secs(30)));
    let host = key("h.example");
    let plan = pool.plan(&host, None);
    assert!(matches!(plan, Plan::Dial(_)));
    pool.forget_if_unused(&host);
    assert_eq!(pool.lock().len(), 1);
    drop(plan);
}

#[test]
fn given_a_host_with_a_dial_in_flight_then_planning_another_host_does_not_forget_it() {
    let pool = Arc::new(Pool::new(Duration::from_secs(30)));
    let first = pool.plan(&key("a.example"), None);
    assert!(matches!(first, Plan::Dial(_)));
    let second = pool.plan(&key("b.example"), None);
    assert!(matches!(second, Plan::Dial(_)));
    assert_eq!(pool.lock().len(), 2);
    drop((first, second));
}

#[test]
fn given_a_host_with_a_live_http1_lease_then_it_is_in_use() {
    let host = Host::new();
    let _lease = LiveGuard::new(&host.live_h1);
    assert!(host.in_use());
}

#[tokio::test]
async fn given_sixteen_full_connections_and_no_dial_in_flight_then_a_send_waits_for_a_stream_without_a_claim()
 {
    let rig = Rig::silent_server().await;
    let mut kept = Vec::new();
    for _ in 0..16 {
        let (state, conn) = rig.land(rig.dialer()).await;
        state.apply_settings(Some(1));
        kept.push((state, conn));
    }
    match rig.shared.pool.plan(&rig.key, None) {
        Plan::Wait(wait) => {
            assert!(matches!(wait.kind, WaitKind::Stream));
            assert!(wait.claim.is_none());
        }
        _ => panic!("the send waits"),
    }
}
