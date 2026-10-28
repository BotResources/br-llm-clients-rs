use std::fmt;
use std::time::Duration;

use br_llm_clients_common_http::Secret;

#[derive(Clone)]
pub struct AccessToken {
    secret: Secret,
    valid_for: Option<Duration>,
}

impl AccessToken {
    pub fn new(secret: Secret, valid_for: Option<Duration>) -> Self {
        Self { secret, valid_for }
    }

    pub fn secret(&self) -> &Secret {
        &self.secret
    }

    pub fn valid_for(&self) -> Option<Duration> {
        self.valid_for
    }
}

impl fmt::Debug for AccessToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AccessToken")
            .field("secret", &self.secret)
            .field("valid_for", &self.valid_for)
            .finish()
    }
}
