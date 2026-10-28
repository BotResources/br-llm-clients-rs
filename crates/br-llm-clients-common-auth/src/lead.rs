use std::sync::{Arc, Mutex};

use br_llm_clients_common_http::AuthorizerError;
use tokio::sync::watch;
use tokio::time::Instant;

use crate::access_token::AccessToken;
use crate::state::{Outcome, Shared, State, lock};

pub(crate) struct Lead {
    state: Arc<Mutex<State>>,
    id: u64,
    started: Instant,
    sender: watch::Sender<Outcome>,
}

impl Lead {
    pub(crate) fn start(state: &Arc<Mutex<State>>, guard: &mut State, started: Instant) -> Self {
        let (id, sender) = guard.open_flight(started);
        Self {
            state: Arc::clone(state),
            id,
            started,
            sender,
        }
    }

    pub(crate) fn finish(self, result: Result<AccessToken, AuthorizerError>) -> Shared {
        let now = Instant::now();
        let settled = lock(&self.state).settle(self.id, self.started, now, result);
        self.sender.send_replace(Some(settled.waiters));
        settled.leader
    }
}
