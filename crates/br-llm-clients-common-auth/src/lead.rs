use std::sync::{Arc, Mutex};

use br_llm_clients_common_http::AuthorizerError;
use tokio::sync::watch;
use tokio::time::Instant;

use crate::access_token::AccessToken;
use crate::state::{Outcome, Shared, State, lock};

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
    ) -> Shared {
        let now = Instant::now();
        let (shared, answer) = lock(&self.state).settle(started, now, result);
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
