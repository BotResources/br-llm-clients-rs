use std::sync::{Mutex, MutexGuard, PoisonError};
use std::task::Waker;

pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

pub(crate) fn wake_all(wakers: Vec<Waker>) {
    for waker in wakers {
        waker.wake();
    }
}

#[derive(Default)]
pub(crate) struct Waiters {
    next: u64,
    slots: Vec<(u64, Waker)>,
}

impl Waiters {
    pub(crate) fn register(&mut self, slot: &mut Option<u64>, waker: &Waker) {
        if let Some(id) = *slot
            && let Some(entry) = self
                .slots
                .iter_mut()
                .find(|(candidate, _)| *candidate == id)
        {
            if !entry.1.will_wake(waker) {
                entry.1 = waker.clone();
            }
            return;
        }
        let id = self.next;
        self.next += 1;
        self.slots.push((id, waker.clone()));
        *slot = Some(id);
    }

    pub(crate) fn cancel(&mut self, slot: Option<u64>) {
        if let Some(id) = slot {
            self.slots.retain(|(candidate, _)| *candidate != id);
        }
    }

    pub(crate) fn take(&mut self) -> Vec<Waker> {
        self.slots.drain(..).map(|(_, waker)| waker).collect()
    }
}
