use std::time::Duration;

use br_llm_clients_common_http::AuthorizerError;
use tokio::sync::watch;
use tokio::time::Instant;

use crate::state::Outcome;

pub(crate) const FETCH_LIMIT: Duration = Duration::from_secs(30);
const FETCH_TIMED_OUT: &str = "the token fetch timed out";

pub(crate) fn timed_out() -> AuthorizerError {
    AuthorizerError::fetch_failed(FETCH_TIMED_OUT)
}

pub(crate) struct Flight {
    pub(crate) id: u64,
    pub(crate) started: Instant,
    pub(crate) receiver: watch::Receiver<Outcome>,
}

impl Flight {
    pub(crate) fn live_for(&self, now: Instant) -> Option<Duration> {
        if self.receiver.has_changed().is_err() {
            return None;
        }
        let left = FETCH_LIMIT.saturating_sub(now.saturating_duration_since(self.started));
        (!left.is_zero()).then_some(left)
    }
}

pub(crate) struct Waiting {
    pub(crate) receiver: watch::Receiver<Outcome>,
    pub(crate) live_for: Duration,
}
