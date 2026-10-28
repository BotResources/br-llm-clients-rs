use std::sync::Arc;

use tokio::sync::watch;

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
    let (sender, receiver) = watch::channel(None);
    drop(sender);
    lock(&cache.state).flight = Some(receiver);
    assert!(matches!(cache.step(), Step::Lead(_)));
}

#[test]
fn given_a_live_flight_when_a_call_steps_then_it_waits() {
    let cache = cache();
    let (_sender, receiver) = watch::channel(None);
    lock(&cache.state).flight = Some(receiver);
    assert!(matches!(cache.step(), Step::Wait(_)));
}

#[test]
fn given_no_flight_when_a_call_steps_then_it_leads() {
    assert!(matches!(cache().step(), Step::Lead(_)));
}
