use crate::acquire_tests::{Outcome, Rig, outcome};

#[tokio::test]
async fn given_a_send_waiting_for_the_first_settings_when_a_zero_and_a_raise_are_read_together_then_it_goes_on_without_losing_an_attempt()
 {
    let rig = Rig::silent_server().await;
    let (state, _conn) = rig.land(rig.dialer()).await;
    let waiter = rig.waiter();
    rig.waiting(1).await;
    state.apply_settings_run(&[Some(0), Some(100)]);
    assert!(matches!(outcome(waiter).await, Outcome::Ready));
}

#[tokio::test]
async fn given_a_send_that_spent_an_attempt_on_a_first_zero_when_it_is_raised_later_then_the_sends_that_start_after_it_use_the_connection()
 {
    let rig = Rig::silent_server().await;
    let (state, _conn) = rig.land(rig.dialer()).await;
    let waited = rig.waiter();
    rig.waiting(1).await;
    state.apply_settings(Some(0));
    assert!(matches!(outcome(waited).await, Outcome::Spent));
    state.apply_settings(Some(3));
    assert!(matches!(outcome(rig.waiter()).await, Outcome::Ready));
    assert_eq!(rig.accepted(), 1);
}
