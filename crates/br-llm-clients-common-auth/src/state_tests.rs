use std::time::Duration;

use br_llm_clients_common_http::{AuthorizerError, Secret};
use tokio::time::Instant;

use crate::access_token::AccessToken;
use crate::state::{Settled, State};

fn token(text: &str, valid_for: Option<Duration>) -> AccessToken {
    AccessToken::new(Secret::new(text).expect("secret"), valid_for)
}

fn text(result: Result<Secret, AuthorizerError>) -> String {
    result.expect("token").expose().to_owned()
}

fn settle_new(
    state: &mut State,
    at: Instant,
    result: Result<AccessToken, AuthorizerError>,
) -> Settled {
    let (id, _sender) = state.open_flight(at);
    state.settle(id, at, at, result)
}

#[test]
fn given_a_token_with_a_validity_when_settled_then_it_is_kept_and_both_answers_are_the_token() {
    let started = Instant::now();
    let mut state = State::default();
    let settled = settle_new(
        &mut state,
        started,
        Ok(token("t", Some(Duration::from_secs(60)))),
    );
    assert_eq!(text(settled.waiters), "t");
    assert_eq!(text(settled.leader), "t");
    assert!(state.held.is_some());
    assert!(state.flight.is_none());
}

#[test]
fn given_a_token_with_no_validity_when_settled_then_it_is_not_kept() {
    let started = Instant::now();
    let mut state = State::default();
    let settled = settle_new(&mut state, started, Ok(token("t", None)));
    assert_eq!(text(settled.waiters), "t");
    assert_eq!(text(settled.leader), "t");
    assert!(state.held.is_none());
}

#[test]
fn given_a_usable_held_token_when_a_failure_is_settled_then_the_leader_gets_the_token_and_the_others_the_error()
 {
    let started = Instant::now();
    let mut state = State::default();
    let _ = settle_new(
        &mut state,
        started,
        Ok(token("old", Some(Duration::from_secs(100)))),
    );
    let error = AuthorizerError::fetch_failed("down");
    let later = started + Duration::from_secs(80);
    let settled = settle_new(&mut state, later, Err(error.clone()));
    assert_eq!(settled.waiters.expect_err("waiters"), error);
    assert_eq!(text(settled.leader), "old");
}

#[test]
fn given_an_ended_held_token_when_a_failure_is_settled_then_both_answers_are_the_error() {
    let started = Instant::now();
    let mut state = State::default();
    let _ = settle_new(
        &mut state,
        started,
        Ok(token("old", Some(Duration::from_secs(100)))),
    );
    let error = AuthorizerError::unavailable("none");
    let later = started + Duration::from_secs(100);
    let settled = settle_new(&mut state, later, Err(error.clone()));
    assert_eq!(settled.waiters.expect_err("waiters"), error);
    assert_eq!(settled.leader.expect_err("leader"), error);
}

#[test]
fn given_a_flight_that_is_no_longer_current_when_it_settles_then_the_state_does_not_change() {
    let started = Instant::now();
    let mut state = State::default();
    let (stale, _old) = state.open_flight(started);
    let (newer, _new) = state.open_flight(started);
    let settled = state.settle(
        stale,
        started,
        started,
        Ok(token("stale", Some(Duration::from_secs(60)))),
    );
    assert_eq!(text(settled.leader), "stale");
    assert!(state.held.is_none());
    assert!(state.is_current(newer));
}
