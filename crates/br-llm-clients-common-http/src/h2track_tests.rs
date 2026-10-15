use std::time::{Duration, Instant};

use super::{Gone, track_streams};
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
async fn given_a_connection_that_ends_before_its_first_settings_then_it_counts_as_silent() {
    let state = bare_state();
    track_streams(
        async { Ok(()) },
        std::sync::Arc::clone(&state),
        Duration::from_secs(60),
    )
    .await;
    assert!(state.is_silent());
    assert!(state.violation().is_none());
    assert!(state.is_retired());
}

#[tokio::test]
async fn given_a_connection_that_ends_after_its_first_settings_then_it_is_not_silent() {
    let state = state_with_limit(10);
    track_streams(
        async { Ok(()) },
        std::sync::Arc::clone(&state),
        Duration::from_secs(60),
    )
    .await;
    assert!(!state.is_silent());
    assert!(state.is_retired());
}
