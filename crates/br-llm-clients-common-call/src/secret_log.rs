use std::collections::VecDeque;
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use br_llm_clients_common_http::Secret;

pub(crate) const LOG_CAPACITY: usize = 1024;

#[derive(Clone, Default)]
pub struct SecretLog {
    kept: Arc<Mutex<VecDeque<Secret>>>,
}

impl SecretLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&self, secret: &Secret) {
        let mut kept = self.lock();
        if kept.iter().any(|known| known.expose() == secret.expose()) {
            return;
        }
        if kept.len() >= LOG_CAPACITY {
            kept.pop_front();
        }
        kept.push_back(secret.clone());
    }

    pub(crate) fn snapshot(&self) -> Vec<Secret> {
        self.lock().iter().cloned().collect()
    }

    fn lock(&self) -> MutexGuard<'_, VecDeque<Secret>> {
        self.kept.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl fmt::Debug for SecretLog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretLog")
    }
}
