use std::time::Duration;

use br_llm_clients_common_http::Secret;
use tokio::time::Instant;

pub(crate) const MAX_VALIDITY: Duration = Duration::from_secs(24 * 60 * 60);
const FULL_SKEW: Duration = Duration::from_secs(300);
const SHORT_VALIDITY: Duration = Duration::from_secs(600);

pub(crate) struct Held {
    secret: Secret,
    started: Instant,
    validity: Duration,
}

impl Held {
    pub(crate) fn new(secret: Secret, started: Instant, valid_for: Duration) -> Self {
        Self {
            secret,
            started,
            validity: valid_for.min(MAX_VALIDITY),
        }
    }

    pub(crate) fn secret(&self) -> &Secret {
        &self.secret
    }

    pub(crate) fn is_fresh(&self, now: Instant) -> bool {
        self.time_left(now) > self.skew()
    }

    pub(crate) fn is_usable(&self, now: Instant) -> bool {
        !self.time_left(now).is_zero()
    }

    fn time_left(&self, now: Instant) -> Duration {
        self.validity
            .saturating_sub(now.saturating_duration_since(self.started))
    }

    fn skew(&self) -> Duration {
        if self.validity < SHORT_VALIDITY {
            self.validity.checked_div(2).unwrap_or_default()
        } else {
            FULL_SKEW
        }
    }
}
