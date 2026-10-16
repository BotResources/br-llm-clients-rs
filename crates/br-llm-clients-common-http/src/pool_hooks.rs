use std::sync::{Arc, Mutex, PoisonError};

type Hook = Arc<dyn Fn() + Send + Sync>;
type Slot = Mutex<Option<Hook>>;

#[derive(Default)]
pub(crate) struct Hooks {
    before_scan: Slot,
    after_scan: Slot,
    before_register: Slot,
}

fn set(slot: &Slot, hook: impl Fn() + Send + Sync + 'static) {
    *slot.lock().unwrap_or_else(PoisonError::into_inner) = Some(Arc::new(hook));
}

fn run(slot: &Slot) {
    let hook = slot.lock().unwrap_or_else(PoisonError::into_inner).clone();
    if let Some(hook) = hook {
        hook();
    }
}

impl Hooks {
    pub(crate) fn on_before_scan(&self, hook: impl Fn() + Send + Sync + 'static) {
        set(&self.before_scan, hook);
    }

    pub(crate) fn on_after_scan(&self, hook: impl Fn() + Send + Sync + 'static) {
        set(&self.after_scan, hook);
    }

    pub(crate) fn on_register(&self, hook: impl Fn() + Send + Sync + 'static) {
        set(&self.before_register, hook);
    }

    pub(crate) fn run_before_scan(&self) {
        run(&self.before_scan);
    }

    pub(crate) fn run_after_scan(&self) {
        run(&self.after_scan);
    }

    pub(crate) fn run_before_register(&self) {
        run(&self.before_register);
    }
}
