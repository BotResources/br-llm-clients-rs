use std::time::{Duration, Instant};

use super::{Gone, track_streams};
use crate::h2state::Standing;
use crate::h2state_tests::{bare_state, state_with_limit};

#[test]
fn given_a_connection_that_ends_then_it_is_retired_before_anyone_is_told_it_is_gone() {
    let state = state_with_limit(10);
    let permit = state.force_acquire();
    let receiver = permit.gone();
    let holding = receiver.borrow();
    let ending = Gone(std::sync::Arc::clone(&state));
    let dropper = std::thread::spawn(move || drop(ending));
    let started = Instant::now();
    while !state.is_retired() {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "the gone signal was sent before the connection was retired"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(!*holding);
    drop(holding);
    dropper.join().expect("dropper");
    assert!(*receiver.borrow());
}

#[tokio::test]
async fn given_a_connection_that_ends_before_its_first_settings_then_a_send_that_waited_for_it_has_lost_an_attempt()
 {
    let state = bare_state();
    track_streams(
        async { Ok(()) },
        std::sync::Arc::clone(&state),
        Duration::from_secs(60),
    )
    .await;
    assert!(state.limit_unknown());
    assert!(state.violation().is_none());
    assert!(state.is_retired());
    assert!(matches!(state.standing(), Standing::Spent));
}

#[tokio::test]
async fn given_a_connection_that_ends_after_its_first_settings_then_it_is_retired_with_its_limit_known()
 {
    let state = state_with_limit(10);
    track_streams(
        async { Ok(()) },
        std::sync::Arc::clone(&state),
        Duration::from_secs(60),
    )
    .await;
    assert!(!state.limit_unknown());
    assert!(state.is_retired());
}

#[tokio::test(start_paused = true)]
async fn given_a_connection_whose_first_settings_do_not_come_within_the_wait_then_it_is_closed_and_spent()
 {
    let state = bare_state();
    let started = tokio::time::Instant::now();
    track_streams(
        std::future::pending::<Result<(), hyper::Error>>(),
        std::sync::Arc::clone(&state),
        Duration::from_secs(7),
    )
    .await;
    assert_eq!(started.elapsed(), Duration::from_secs(7));
    assert!(state.is_retired());
    assert!(matches!(state.standing(), Standing::Spent));
}

#[tokio::test(start_paused = true)]
async fn given_a_connection_whose_first_settings_came_then_it_is_not_closed_when_the_wait_passes() {
    let state = state_with_limit(10);
    let tracked = track_streams(
        std::future::pending::<Result<(), hyper::Error>>(),
        std::sync::Arc::clone(&state),
        Duration::from_secs(7),
    );
    tokio::pin!(tracked);
    tokio::select! {
        () = &mut tracked => panic!("the connection was closed"),
        () = tokio::time::sleep(Duration::from_secs(60)) => {}
    }
    assert!(!state.is_retired());
}
