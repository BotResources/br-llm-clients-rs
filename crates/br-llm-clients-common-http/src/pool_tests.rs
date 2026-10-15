use std::sync::Arc;
use std::time::Duration;

use tokio::net::TcpListener;

use crate::connect::open;
use crate::dialer::DialFailure;
use crate::endpoint::Endpoint;
use crate::h2state_tests::bare_state;
use crate::pool::{Plan, Pool, PoolKey, WaitKind};
use crate::transport::bump;
use crate::tuning::Tuning;

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

fn pool() -> Arc<Pool> {
    Arc::new(Pool::new(Duration::from_secs(30)))
}

fn connect_failure() -> DialFailure {
    DialFailure::Connect("refused".to_owned())
}

#[test]
fn given_a_wake_after_the_permit_scan_when_planning_a_wait_then_the_wait_already_sees_it() {
    let pool = pool();
    let host = key("h.example");
    let Plan::Dial(dialer) = pool.plan(&host, None) else {
        panic!("the first send dials");
    };
    let signal = Arc::clone(&pool.lock().get(&host).expect("host").signal);
    pool.run_after_scan(move || bump(&signal));
    let Plan::Wait(wait) = pool.plan(&host, None) else {
        panic!("the second send waits");
    };
    assert!(wait.changed.has_changed().expect("open"));
    drop(dialer);
}

#[test]
fn given_a_waiter_when_the_dial_it_waited_for_fails_then_it_gets_that_failure_and_does_not_dial() {
    let pool = pool();
    let host = key("h.example");
    let Plan::Dial(dialer) = pool.plan(&host, None) else {
        panic!("the first send dials");
    };
    let Plan::Wait(wait) = pool.plan(&host, None) else {
        panic!("the second send waits");
    };
    dialer.fail(connect_failure());
    assert!(matches!(
        pool.plan(&host, Some(wait.seq)),
        Plan::Failed(DialFailure::Connect(_))
    ));
}

#[test]
fn given_an_old_failure_and_a_dial_dropped_without_outcome_when_a_waiter_plans_again_then_it_dials()
{
    let pool = pool();
    let host = key("h.example");
    let Plan::Dial(first) = pool.plan(&host, None) else {
        panic!("the first send dials");
    };
    first.fail(connect_failure());
    let Plan::Dial(second) = pool.plan(&host, None) else {
        panic!("the next send dials");
    };
    let Plan::Wait(wait) = pool.plan(&host, None) else {
        panic!("the third send waits");
    };
    drop(second);
    assert!(matches!(pool.plan(&host, Some(wait.seq)), Plan::Dial(_)));
}

#[tokio::test]
async fn given_a_failed_dial_and_then_a_successful_one_when_it_registers_then_no_failure_is_kept() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let endpoint = Endpoint::plain_http(&format!(
        "http://{}/x",
        listener.local_addr().expect("addr")
    ))
    .expect("endpoint");
    let pool = pool();
    let host = key("h.example");
    let Plan::Dial(failing) = pool.plan(&host, None) else {
        panic!("the first send dials");
    };
    failing.fail(connect_failure());
    assert!(pool.lock().get(&host).expect("host").has_failure());
    let Plan::Dial(succeeding) = pool.plan(&host, None) else {
        panic!("the next send dials");
    };
    let tls = crate::tls::build_connector(&[]).expect("connector");
    let Ok(opened) = open(
        &tls,
        &endpoint,
        &succeeding.host,
        Tuning::FIXED,
        Duration::from_secs(5),
    )
    .await
    else {
        panic!("the connection opens");
    };
    let conn = succeeding.register(opened);
    assert!(conn.is_some());
    assert!(!pool.lock().get(&host).expect("host").has_failure());
}

#[test]
fn given_a_host_that_only_holds_a_recorded_failure_when_planning_another_then_it_is_forgotten() {
    let pool = pool();
    let Plan::Dial(dialer) = pool.plan(&key("a.example"), None) else {
        panic!("the first send dials");
    };
    dialer.fail(connect_failure());
    assert!(matches!(pool.plan(&key("b.example"), None), Plan::Dial(_)));
    assert_eq!(pool.lock().len(), 1);
}

#[test]
fn given_a_waiter_still_to_read_a_failure_when_planning_another_host_then_the_failing_host_is_kept_until_it_is_read()
 {
    let pool = pool();
    let host = key("a.example");
    let Plan::Dial(dialer) = pool.plan(&host, None) else {
        panic!("the first send dials");
    };
    let Plan::Wait(wait) = pool.plan(&host, None) else {
        panic!("the second send waits");
    };
    dialer.fail(connect_failure());
    drop(pool.plan(&key("b.example"), None));
    assert_eq!(pool.lock().len(), 2);
    assert!(matches!(pool.plan(&host, Some(wait.seq)), Plan::Failed(_)));
}

fn landing_waited_for(plan: Plan) -> std::sync::Arc<crate::claim::Landing> {
    match plan {
        Plan::Wait(wait) => match wait.kind {
            WaitKind::Dial(landing) => landing,
            WaitKind::Stream => panic!("the send waits for a dial"),
        },
        _ => panic!("the send waits"),
    }
}

#[test]
fn given_a_dial_in_flight_when_a_send_waits_then_it_waits_for_the_landing_of_that_dial() {
    let pool = pool();
    let host = key("h.example");
    let Plan::Dial(dialer) = pool.plan(&host, None) else {
        panic!("the first send dials");
    };
    let landing = landing_waited_for(pool.plan(&host, None));
    let state = bare_state();
    assert!(landing.landed().is_none());
    let held = pool
        .lock()
        .get(&host)
        .map(|h| std::sync::Arc::clone(&h.landing));
    assert!(held.is_some_and(|held| std::sync::Arc::ptr_eq(&held, &landing)));
    landing.land(&state);
    assert!(landing.landed().is_some());
    drop(dialer);
}

#[test]
fn given_a_later_dial_when_a_send_waits_then_it_does_not_wait_for_the_landing_of_an_earlier_dial() {
    let pool = pool();
    let host = key("h.example");
    let Plan::Dial(first) = pool.plan(&host, None) else {
        panic!("the first send dials");
    };
    let earlier = landing_waited_for(pool.plan(&host, None));
    earlier.land(&bare_state());
    drop(first);
    let Plan::Dial(second) = pool.plan(&host, None) else {
        panic!("the next send dials");
    };
    let later = landing_waited_for(pool.plan(&host, None));
    assert!(!std::sync::Arc::ptr_eq(&earlier, &later));
    assert!(later.landed().is_none());
    drop(second);
}
