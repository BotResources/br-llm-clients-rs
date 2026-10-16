use std::future::poll_fn;
use std::task::Poll;
use std::time::Duration;

use crate::acquire_tests::{Acquiring, Outcome, Rig, outcome};
use crate::dialer::DialFailure;
use crate::pool::Plan;
use crate::send_error::SendError;

const SERVED_WITHIN: Duration = Duration::from_secs(3);

async fn polled_once(acquisition: &mut Acquiring) {
    poll_fn(|context| {
        assert!(acquisition.as_mut().poll(context).is_pending());
        Poll::Ready(())
    })
    .await;
}

async fn served(rig: &Rig) -> Outcome {
    tokio::time::timeout(SERVED_WITHIN, outcome(rig.waiter()))
        .await
        .expect("the send is held up by a dial that no one drives")
}

#[tokio::test]
async fn given_a_cold_host_and_a_send_that_started_the_dial_and_is_not_polled_again_then_the_sends_that_arrive_are_served_by_that_dial()
 {
    let rig = Rig::announcing_server(100).await;
    let mut stalled = rig.acquisition();
    polled_once(&mut stalled).await;
    assert!(matches!(served(&rig).await, Outcome::Ready));
    assert!(matches!(served(&rig).await, Outcome::Ready));
    assert_eq!(rig.accepted(), 1);
    assert_eq!(rig.shared.pool.h2_connections(), 1);
    drop(stalled);
    assert!(matches!(served(&rig).await, Outcome::Ready));
    assert_eq!(rig.accepted(), 1);
    assert_eq!(rig.shared.pool.h2_connections(), 1);
}

#[tokio::test]
async fn given_a_cold_host_and_a_send_that_started_the_dial_when_it_is_polled_again_then_it_gets_the_connection_of_that_dial()
 {
    let rig = Rig::announcing_server(100).await;
    let mut stalled = rig.acquisition();
    polled_once(&mut stalled).await;
    assert!(matches!(served(&rig).await, Outcome::Ready));
    assert!(matches!(
        stalled.await,
        Ok(crate::acquire::Acquisition::Ready(_))
    ));
    assert_eq!(rig.accepted(), 1);
    assert_eq!(rig.shared.pool.h2_connections(), 1);
}

#[tokio::test]
async fn given_a_busy_host_and_a_send_that_started_the_dial_for_one_more_connection_and_is_not_polled_again_then_the_sends_that_arrive_are_served()
 {
    let rig = Rig::announcing_server(1).await;
    let (full, _held) = rig.land(rig.dialer()).await;
    full.apply_settings(Some(1));
    let mut stalled = rig.acquisition();
    polled_once(&mut stalled).await;
    assert!(matches!(served(&rig).await, Outcome::Ready));
    assert_eq!(rig.accepted(), 3);
    drop(stalled);
}

#[tokio::test]
async fn given_a_send_that_started_a_dial_when_it_is_dropped_then_the_dial_stops_and_the_host_can_be_dialed_again()
 {
    let rig = Rig::slow_silent_server(Duration::from_secs(30), Duration::from_secs(60)).await;
    let mut stalled = rig.acquisition();
    polled_once(&mut stalled).await;
    assert!(rig.dialing());
    drop(stalled);
    let started = tokio::time::Instant::now();
    while rig.dialing() {
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "the dial of a dropped send still holds the gate"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(matches!(
        rig.shared.pool.plan(&rig.key, None),
        Plan::Dial(_)
    ));
}

#[tokio::test]
async fn given_a_dial_for_one_more_connection_that_fails_then_the_sends_that_waited_for_it_fail_with_that_failure_and_do_not_dial()
 {
    let rig = Rig::announcing_server(1).await;
    let (full, _held) = rig.land(rig.dialer()).await;
    full.apply_settings(Some(1));
    let capacity = rig.dialer();
    let waiters: Vec<_> = (0..5).map(|_| rig.waiter()).collect();
    rig.waiting(5).await;
    capacity.fail(DialFailure::Tls("the handshake failed".to_owned()));
    for waiter in waiters {
        let Outcome::Failed(error) = outcome(waiter).await else {
            panic!("the waiter fails with the dial failure");
        };
        assert!(matches!(error, SendError::Tls { .. }), "{error:?}");
    }
    assert_eq!(rig.accepted(), 1);
}

#[tokio::test]
async fn given_a_dial_for_one_more_connection_that_failed_when_another_send_arrives_then_it_dials_for_itself()
 {
    let rig = Rig::announcing_server(1).await;
    let (full, _held) = rig.land(rig.dialer()).await;
    full.apply_settings(Some(1));
    rig.dialer()
        .fail(DialFailure::Connect("refused".to_owned()));
    assert!(matches!(
        rig.shared.pool.plan(&rig.key, None),
        Plan::Dial(_)
    ));
}

#[tokio::test]
async fn given_a_dial_for_one_more_connection_that_timed_out_then_the_sends_that_waited_for_it_fail_with_connect_timeout()
 {
    let rig = Rig::announcing_server(1).await;
    let (full, _held) = rig.land(rig.dialer()).await;
    full.apply_settings(Some(1));
    let capacity = rig.dialer();
    let waiter = rig.waiter();
    rig.waiting(1).await;
    capacity.fail(DialFailure::Timeout);
    let Outcome::Failed(error) = outcome(waiter).await else {
        panic!("the waiter fails");
    };
    assert!(
        matches!(error, SendError::ConnectTimeout { .. }),
        "{error:?}"
    );
}
