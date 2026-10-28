use std::sync::{Arc, Mutex};

use br_llm_clients_common_http::AuthorizerError;
use tokio::sync::watch;
use tokio::time::Instant;

use crate::access_token::AccessToken;
use crate::state::{Outcome, Shared, State, lock};

pub(crate) struct Lead {
    state: Arc<Mutex<State>>,
    sender: watch::Sender<Outcome>,
}

impl Lead {
    pub(crate) fn start(state: &Arc<Mutex<State>>, guard: &mut State) -> Self {
        let (sender, receiver) = watch::channel(None);
        guard.flight = Some(receiver);
        Self {
            state: Arc::clone(state),
            sender,
        }
    }

    pub(crate) fn finish(
        self,
        started: Instant,
        result: Result<AccessToken, AuthorizerError>,
    ) -> Shared {
        let now = Instant::now();
        let (shared, answer) = lock(&self.state).settle(started, now, result);
        self.sender.send_replace(Some(shared));
        answer
    }
}
