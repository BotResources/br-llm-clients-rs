use std::fmt;
use std::sync::{Arc, Mutex};

use br_llm_clients_common_http::{AuthorizerError, Secret};
use tokio::sync::watch;
use tokio::time::{Instant, timeout};

use crate::flight::{FETCH_LIMIT, Waiting, timed_out};
use crate::lead::Lead;
use crate::pass::pass;
use crate::state::{Outcome, State, lock};
use crate::token_source::TokenSource;

pub(crate) enum Step {
    Ready(Secret),
    Wait(Waiting),
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
        let mut lost = None;
        loop {
            match self.step() {
                Step::Ready(secret) => return Ok(secret),
                Step::Wait(waiting) => {
                    if lost == Some(waiting.id) {
                        pass().await;
                    }
                    match timeout(waiting.time_left, wait(waiting.receiver)).await {
                        Ok(Some(result)) => return result,
                        Ok(None) => lost = Some(waiting.id),
                        Err(_) => return Err(timed_out()),
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
        let live = state.flight.as_ref().and_then(|flight| {
            flight.time_left(now).map(|time_left| Waiting {
                id: flight.id,
                receiver: flight.receiver.clone(),
                time_left,
            })
        });
        if let Some(waiting) = live {
            return match state.held.as_ref().filter(|held| held.is_usable(now)) {
                Some(held) => Step::Ready(held.secret().clone()),
                None => Step::Wait(waiting),
            };
        }
        Step::Lead(Lead::start(&self.state, &mut state, now))
    }

    async fn fetch(&self, lead: Lead) -> Result<Secret, AuthorizerError> {
        let result = timeout(FETCH_LIMIT, self.source.fetch())
            .await
            .unwrap_or_else(|_| Err(timed_out()));
        lead.finish(result)
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
