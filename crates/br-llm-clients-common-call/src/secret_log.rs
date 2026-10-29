use std::collections::VecDeque;
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use br_llm_clients_common_http::Secret;

pub(crate) const LOG_CAPACITY: usize = 1024;

#[derive(Default)]
struct Kept {
    secrets: VecDeque<Secret>,
    generation: u64,
}

#[derive(Clone, Default)]
pub struct SecretLog {
    kept: Arc<Mutex<Kept>>,
}

impl SecretLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&self, secret: &Secret) {
        let mut kept = self.lock();
        if kept
            .secrets
            .iter()
            .any(|known| known.expose() == secret.expose())
        {
            return;
        }
        if kept.secrets.len() >= LOG_CAPACITY {
            kept.secrets.pop_front();
        }
        kept.secrets.push_back(secret.clone());
        kept.generation = kept.generation.wrapping_add(1);
    }

    #[cfg(test)]
    pub(crate) fn snapshot(&self) -> Vec<Secret> {
        self.versioned().1
    }

    pub(crate) fn generation(&self) -> u64 {
        self.lock().generation
    }

    pub(crate) fn versioned(&self) -> (u64, Vec<Secret>) {
        let kept = self.lock();
        (kept.generation, kept.secrets.iter().cloned().collect())
    }

    fn lock(&self) -> MutexGuard<'_, Kept> {
        self.kept.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl fmt::Debug for SecretLog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretLog")
    }
}
