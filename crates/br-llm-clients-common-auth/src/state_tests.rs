use std::time::Duration;

use br_llm_clients_common_http::{AuthorizerError, Secret};
use tokio::time::Instant;

use crate::access_token::AccessToken;
use crate::state::State;

fn token(text: &str, valid_for: Option<Duration>) -> AccessToken {
    AccessToken::new(Secret::new(text).expect("secret"), valid_for)
}

fn text(result: Result<Secret, AuthorizerError>) -> String {
    result.expect("token").expose().to_owned()
}

#[test]
fn given_a_token_with_a_validity_when_settled_then_it_is_kept_and_both_answers_are_the_token() {
    let started = Instant::now();
    let mut state = State::default();
    let (shared, answer) = state.settle(
        started,
        started,
        Ok(token("t", Some(Duration::from_secs(60)))),
    );
    assert_eq!(text(shared), "t");
    assert_eq!(text(answer), "t");
    assert!(state.held.is_some());
}

#[test]
fn given_a_token_with_no_validity_when_settled_then_it_is_not_kept() {
    let started = Instant::now();
    let mut state = State::default();
    let (shared, answer) = state.settle(started, started, Ok(token("t", None)));
    assert_eq!(text(shared), "t");
    assert_eq!(text(answer), "t");
    assert!(state.held.is_none());
}

#[test]
fn given_a_usable_held_token_when_a_failure_is_settled_then_the_leader_gets_the_token_and_the_others_the_error()
 {
    let started = Instant::now();
    let mut state = State::default();
    let _ = state.settle(
        started,
        started,
        Ok(token("old", Some(Duration::from_secs(100)))),
    );
    let error = AuthorizerError::fetch_failed("down");
    let later = started + Duration::from_secs(80);
    let (shared, answer) = state.settle(later, later, Err(error.clone()));
    assert_eq!(shared.expect_err("shared"), error);
    assert_eq!(text(answer), "old");
}

#[test]
fn given_an_ended_held_token_when_a_failure_is_settled_then_both_answers_are_the_error() {
    let started = Instant::now();
    let mut state = State::default();
    let _ = state.settle(
        started,
        started,
        Ok(token("old", Some(Duration::from_secs(100)))),
    );
    let error = AuthorizerError::unavailable("none");
    let later = started + Duration::from_secs(100);
    let (shared, answer) = state.settle(later, later, Err(error.clone()));
    assert_eq!(shared.expect_err("shared"), error);
    assert_eq!(answer.expect_err("answer"), error);
}

#[test]
fn given_a_flight_when_settled_then_the_flight_is_cleared() {
    let started = Instant::now();
    let mut state = State::default();
    let (_sender, receiver) = tokio::sync::watch::channel(None);
    state.flight = Some(receiver);
    let _ = state.settle(started, started, Err(AuthorizerError::fetch_failed("x")));
    assert!(state.flight.is_none());
}
