use std::sync::Arc;
use std::time::Duration;

use tokio::time::Instant;

use crate::flight::FETCH_LIMIT;
use crate::state::lock;
use crate::static_token::StaticToken;
use crate::token_cache::{Step, TokenCache};
use br_llm_clients_common_http::Secret;

fn cache() -> TokenCache {
    TokenCache::new(Arc::new(StaticToken::new(
        Secret::new("t").expect("secret"),
    )))
}

#[test]
fn given_a_flight_whose_leader_is_gone_when_a_call_steps_then_it_leads_and_does_not_wait() {
    let cache = cache();
    let (_, sender) = lock(&cache.state).open_flight(Instant::now());
    drop(sender);
    assert!(matches!(cache.step(), Step::Lead(_)));
}

#[test]
fn given_a_live_flight_when_a_call_steps_then_it_waits_for_the_time_the_flight_has_left() {
    let cache = cache();
    let started = Instant::now() - Duration::from_secs(10);
    let (_, _sender) = lock(&cache.state).open_flight(started);
    match cache.step() {
        Step::Wait(waiting) => {
            assert!(waiting.time_left <= Duration::from_secs(20));
            assert!(waiting.time_left > Duration::from_secs(19));
        }
        _ => panic!("the call did not wait"),
    }
}

#[test]
fn given_a_flight_past_its_limit_when_a_call_steps_then_it_leads_a_new_flight() {
    let cache = cache();
    let started = Instant::now() - FETCH_LIMIT;
    let (old, _sender) = lock(&cache.state).open_flight(started);
    assert!(matches!(cache.step(), Step::Lead(_)));
    assert!(!lock(&cache.state).is_current(old));
}

#[test]
fn given_no_flight_when_a_call_steps_then_it_leads() {
    assert!(matches!(cache().step(), Step::Lead(_)));
}
