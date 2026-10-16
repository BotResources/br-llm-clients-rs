use std::future::{Future, pending};
use std::pin::pin;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use crate::driver::drive;
use crate::idle::IdleClock;

const LIMIT: Duration = Duration::from_secs(1);

#[tokio::test(start_paused = true)]
async fn given_an_expiry_and_a_disarm_that_are_ready_together_then_the_connection_is_never_dropped()
{
    for trial in 0..64 {
        let (clock, watch) = IdleClock::new(LIMIT);
        clock.arm();
        let mut driver = pin!(drive(pending::<()>(), watch));
        let mut cx = Context::from_waker(Waker::noop());
        assert!(driver.as_mut().poll(&mut cx).is_pending());
        tokio::time::advance(LIMIT * 2).await;
        assert!(clock.disarm());
        assert!(driver.as_mut().poll(&mut cx).is_pending(), "trial {trial}");
        assert!(!clock.is_armed());
    }
}

#[tokio::test(start_paused = true)]
async fn given_an_expiry_and_a_new_arming_that_are_ready_together_then_the_connection_waits_for_the_new_one()
 {
    for trial in 0..64 {
        let (clock, watch) = IdleClock::new(LIMIT);
        clock.arm();
        let mut driver = pin!(drive(pending::<()>(), watch));
        let mut cx = Context::from_waker(Waker::noop());
        assert!(driver.as_mut().poll(&mut cx).is_pending());
        tokio::time::advance(LIMIT * 2).await;
        assert!(clock.disarm());
        clock.arm();
        assert!(driver.as_mut().poll(&mut cx).is_pending(), "trial {trial}");
    }
}

#[tokio::test(start_paused = true)]
async fn given_an_idle_connection_when_the_limit_passes_then_the_driver_ends_and_the_connection_cannot_be_taken()
 {
    let (clock, watch) = IdleClock::new(LIMIT);
    clock.arm();
    let mut driver = pin!(drive(pending::<()>(), watch));
    let mut cx = Context::from_waker(Waker::noop());
    assert!(driver.as_mut().poll(&mut cx).is_pending());
    tokio::time::advance(LIMIT * 2).await;
    assert_eq!(driver.as_mut().poll(&mut cx), Poll::Ready(()));
    assert!(!clock.disarm());
    clock.arm();
    assert!(!clock.is_armed());
}

#[tokio::test(start_paused = true)]
async fn given_a_connection_taken_before_the_limit_then_the_driver_keeps_waiting() {
    let (clock, watch) = IdleClock::new(LIMIT);
    clock.arm();
    let mut driver = pin!(drive(pending::<()>(), watch));
    let mut cx = Context::from_waker(Waker::noop());
    assert!(driver.as_mut().poll(&mut cx).is_pending());
    assert!(clock.disarm());
    tokio::time::advance(LIMIT * 5).await;
    assert!(driver.as_mut().poll(&mut cx).is_pending());
}
