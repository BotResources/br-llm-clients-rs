use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use br_llm_clients_common_http::{AuthorizerError, Secret};
use tokio::sync::watch;
use tokio::time::Instant;

use crate::access_token::AccessToken;
use crate::held::Held;

pub(crate) type Outcome = Option<Result<Secret, AuthorizerError>>;

#[derive(Default)]
pub(crate) struct State {
    pub(crate) held: Option<Held>,
    pub(crate) flight: Option<watch::Receiver<Outcome>>,
}

pub(crate) fn lock(state: &Mutex<State>) -> MutexGuard<'_, State> {
    state.lock().unwrap_or_else(PoisonError::into_inner)
}

pub(crate) struct Lead {
    state: Arc<Mutex<State>>,
    sender: Option<watch::Sender<Outcome>>,
}

impl Lead {
    pub(crate) fn start(state: &Arc<Mutex<State>>, guard: &mut State) -> Self {
        let (sender, receiver) = watch::channel(None);
        guard.flight = Some(receiver);
        Self {
            state: Arc::clone(state),
            sender: Some(sender),
        }
    }

    pub(crate) fn finish(
        mut self,
        started: Instant,
        result: Result<AccessToken, AuthorizerError>,
    ) -> Result<Secret, AuthorizerError> {
        let now = Instant::now();
        let (shared, answer) = {
            let mut state = lock(&self.state);
            state.flight = None;
            match result {
                Ok(token) => {
                    state.held = token
                        .valid_for()
                        .map(|valid_for| Held::new(token.secret().clone(), started, valid_for));
                    let secret = token.secret().clone();
                    (Ok(secret.clone()), Ok(secret))
                }
                Err(error) => {
                    let fallback = state
                        .held
                        .as_ref()
                        .filter(|held| held.is_usable(now))
                        .map(|held| held.secret().clone());
                    let answer = fallback.ok_or_else(|| error.clone());
                    (Err(error), answer)
                }
            }
        };
        if let Some(sender) = self.sender.take() {
            sender.send_replace(Some(shared));
        }
        answer
    }
}

impl Drop for Lead {
    fn drop(&mut self) {
        if self.sender.is_some() {
            lock(&self.state).flight = None;
        }
    }
}
