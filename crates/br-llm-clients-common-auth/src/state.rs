use std::sync::{Mutex, MutexGuard, PoisonError};

use br_llm_clients_common_http::{AuthorizerError, Secret};
use tokio::sync::watch;
use tokio::time::Instant;

use crate::access_token::AccessToken;
use crate::held::Held;

pub(crate) type Shared = Result<Secret, AuthorizerError>;
pub(crate) type Outcome = Option<Shared>;

#[derive(Default)]
pub(crate) struct State {
    pub(crate) held: Option<Held>,
    pub(crate) flight: Option<watch::Receiver<Outcome>>,
}

pub(crate) fn lock(state: &Mutex<State>) -> MutexGuard<'_, State> {
    state.lock().unwrap_or_else(PoisonError::into_inner)
}

impl State {
    pub(crate) fn settle(
        &mut self,
        started: Instant,
        now: Instant,
        result: Result<AccessToken, AuthorizerError>,
    ) -> (Shared, Shared) {
        self.flight = None;
        match result {
            Ok(token) => {
                self.held = token
                    .valid_for()
                    .map(|valid_for| Held::new(token.secret().clone(), started, valid_for));
                let secret = token.secret().clone();
                (Ok(secret.clone()), Ok(secret))
            }
            Err(error) => {
                let fallback = self
                    .held
                    .as_ref()
                    .filter(|held| held.is_usable(now))
                    .map(|held| held.secret().clone());
                let answer = fallback.ok_or_else(|| error.clone());
                (Err(error), answer)
            }
        }
    }
}
