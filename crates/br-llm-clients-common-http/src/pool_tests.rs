use std::sync::Arc;
use std::time::Duration;

use crate::endpoint::Endpoint;
use crate::pool::{Plan, Pool, PoolKey};

fn key(host: &str) -> PoolKey {
    PoolKey::of(&Endpoint::https(&format!("https://{host}/")).expect("endpoint"))
}

#[test]
fn given_hosts_with_nothing_in_them_when_planning_another_then_they_are_forgotten() {
    let pool = Arc::new(Pool::new(Duration::from_secs(30)));
    for index in 0..50 {
        let plan = pool.plan(&key(&format!("h{index}.example")), None);
        assert!(matches!(plan, Plan::Dial(_)));
        drop(plan);
    }
    assert_eq!(pool.lock().len(), 1);
}

#[test]
fn given_a_dial_in_flight_when_planning_the_same_host_then_the_send_waits_for_it() {
    let pool = Arc::new(Pool::new(Duration::from_secs(30)));
    let first = pool.plan(&key("h.example"), None);
    assert!(matches!(first, Plan::Dial(_)));
    let second = pool.plan(&key("h.example"), None);
    assert!(matches!(second, Plan::Wait(..)));
}

#[test]
fn given_a_dial_dropped_before_it_finished_when_planning_again_then_the_next_send_dials() {
    let pool = Arc::new(Pool::new(Duration::from_secs(30)));
    drop(pool.plan(&key("h.example"), None));
    assert!(matches!(pool.plan(&key("h.example"), None), Plan::Dial(_)));
}
