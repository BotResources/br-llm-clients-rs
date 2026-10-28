use std::sync::{Mutex, MutexGuard, PoisonError};

use br_llm_clients_common_http::{AuthorizerError, Secret};
use tokio::sync::watch;
use tokio::time::Instant;

use crate::access_token::AccessToken;
use crate::flight::Flight;
use crate::held::Held;

pub(crate) type Shared = Result<Secret, AuthorizerError>;
pub(crate) type Outcome = Option<Shared>;

#[derive(Default)]
pub(crate) struct State {
    pub(crate) held: Option<Held>,
    pub(crate) flight: Option<Flight>,
    next_flight: u64,
}

pub(crate) struct Settled {
    pub(crate) waiters: Shared,
    pub(crate) leader: Shared,
}

pub(crate) fn lock(state: &Mutex<State>) -> MutexGuard<'_, State> {
    state.lock().unwrap_or_else(PoisonError::into_inner)
}

impl State {
    pub(crate) fn open_flight(&mut self, started: Instant) -> (u64, watch::Sender<Outcome>) {
        let id = self.next_flight;
        self.next_flight = id.wrapping_add(1);
        let (sender, receiver) = watch::channel(None);
        self.flight = Some(Flight {
            id,
            started,
            receiver,
        });
        (id, sender)
    }

    pub(crate) fn is_current(&self, id: u64) -> bool {
        self.flight.as_ref().is_some_and(|flight| flight.id == id)
    }

    pub(crate) fn expire(&mut self, id: u64) {
        if self.is_current(id) {
            self.flight = None;
        }
    }

    pub(crate) fn settle(
        &mut self,
        id: u64,
        started: Instant,
        now: Instant,
        result: Result<AccessToken, AuthorizerError>,
    ) -> Settled {
        let current = self.is_current(id);
        if current {
            self.flight = None;
        }
        match result {
            Ok(token) => {
                if current {
                    self.held = token
                        .valid_for()
                        .map(|valid_for| Held::new(token.secret().clone(), started, valid_for));
                }
                Settled {
                    waiters: Ok(token.secret().clone()),
                    leader: Ok(token.secret().clone()),
                }
            }
            Err(error) => {
                let fallback = self
                    .held
                    .as_ref()
                    .filter(|held| held.is_usable(now))
                    .map(|held| held.secret().clone());
                Settled {
                    leader: fallback.ok_or_else(|| error.clone()),
                    waiters: Err(error),
                }
            }
        }
    }
}
