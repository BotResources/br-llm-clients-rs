use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use br_llm_clients_common_http::{AuthorizerError, Secret};
use tokio::sync::watch;
use tokio::time::{Instant, timeout};

use crate::lead::Lead;
use crate::state::{Outcome, State, lock};
use crate::token_source::TokenSource;

const FETCH_LIMIT: Duration = Duration::from_secs(30);
const FETCH_TIMED_OUT: &str = "the token fetch timed out";

pub(crate) enum Step {
    Ready(Secret),
    Wait(watch::Receiver<Outcome>),
    Lead(Lead),
}

#[derive(Clone)]
pub struct TokenCache {
    source: Arc<dyn TokenSource>,
    pub(crate) state: Arc<Mutex<State>>,
}

impl TokenCache {
    pub fn new(source: Arc<dyn TokenSource>) -> Self {
        Self {
            source,
            state: Arc::new(Mutex::new(State::default())),
        }
    }

    pub fn secrets(&self) -> Vec<Secret> {
        self.source.secrets()
    }

    pub async fn token(&self) -> Result<Secret, AuthorizerError> {
        loop {
            match self.step() {
                Step::Ready(secret) => return Ok(secret),
                Step::Wait(receiver) => {
                    if let Some(result) = wait(receiver).await {
                        return result;
                    }
                }
                Step::Lead(lead) => return self.fetch(lead).await,
            }
        }
    }

    pub(crate) fn step(&self) -> Step {
        let now = Instant::now();
        let mut state = lock(&self.state);
        if let Some(held) = state.held.as_ref().filter(|held| held.is_fresh(now)) {
            return Step::Ready(held.secret().clone());
        }
        let live = state
            .flight
            .clone()
            .filter(|receiver| receiver.has_changed().is_ok());
        if let Some(receiver) = live {
            return match state.held.as_ref().filter(|held| held.is_usable(now)) {
                Some(held) => Step::Ready(held.secret().clone()),
                None => Step::Wait(receiver),
            };
        }
        Step::Lead(Lead::start(&self.state, &mut state))
    }

    async fn fetch(&self, lead: Lead) -> Result<Secret, AuthorizerError> {
        let started = Instant::now();
        let result = match timeout(FETCH_LIMIT, self.source.fetch()).await {
            Ok(result) => result,
            Err(_) => Err(AuthorizerError::fetch_failed(FETCH_TIMED_OUT)),
        };
        lead.finish(started, result)
    }
}

async fn wait(mut receiver: watch::Receiver<Outcome>) -> Outcome {
    receiver
        .wait_for(Option::is_some)
        .await
        .ok()
        .and_then(|outcome| outcome.clone())
}

impl fmt::Debug for TokenCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TokenCache").finish_non_exhaustive()
    }
}
