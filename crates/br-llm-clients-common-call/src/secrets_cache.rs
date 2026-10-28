use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::call_spec::CallSpec;
use crate::text_clean::Secrets;

struct Held {
    generation: u64,
    secrets: Arc<Secrets>,
}

#[derive(Default)]
pub(crate) struct SecretsCache {
    held: Mutex<Option<Held>>,
    #[cfg(test)]
    taken: std::sync::atomic::AtomicUsize,
}

impl SecretsCache {
    pub(crate) fn in_effect(&self, spec: &CallSpec) -> Arc<Secrets> {
        let mut held = self.lock();
        let generation = spec.secret_log.generation();
        if let Some(current) = held.as_ref().filter(|held| held.generation == generation) {
            return Arc::clone(&current.secrets);
        }
        let (generation, logged) = spec.secret_log.versioned();
        let secrets = Arc::new(spec.secrets_with(logged));
        *held = Some(Held {
            generation,
            secrets: Arc::clone(&secrets),
        });
        #[cfg(test)]
        self.taken.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        secrets
    }

    #[cfg(test)]
    pub(crate) fn snapshots_taken(&self) -> usize {
        self.taken.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn lock(&self) -> MutexGuard<'_, Option<Held>> {
        self.held.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
