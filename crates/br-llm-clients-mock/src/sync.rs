use std::future::Future;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::pin::Pin;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, Waker};

pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

pub(crate) fn wake_all(wakers: Vec<Waker>) {
    let mut first_panic = None;
    for waker in wakers {
        if let Err(payload) = catch_unwind(AssertUnwindSafe(|| waker.wake())) {
            first_panic.get_or_insert(payload);
        }
    }
    if let Some(payload) = first_panic
        && !std::thread::panicking()
    {
        resume_unwind(payload);
    }
}

#[derive(Default)]
pub(crate) struct Waiters {
    next: u64,
    slots: Vec<(u64, Waker)>,
}

impl Waiters {
    #[must_use = "the displaced waker must be dropped after the lock is released"]
    pub(crate) fn register(&mut self, slot: &mut Option<u64>, waker: Waker) -> Option<Waker> {
        if let Some(id) = *slot
            && let Some(entry) = self
                .slots
                .iter_mut()
                .find(|(candidate, _)| *candidate == id)
        {
            if entry.1.will_wake(&waker) {
                return Some(waker);
            }
            return Some(std::mem::replace(&mut entry.1, waker));
        }
        let id = self.next;
        self.next += 1;
        self.slots.push((id, waker));
        *slot = Some(id);
        None
    }

    #[must_use = "the removed waker must be dropped after the lock is released"]
    pub(crate) fn cancel(&mut self, slot: Option<u64>) -> Option<Waker> {
        let id = slot?;
        let position = self
            .slots
            .iter()
            .position(|(candidate, _)| *candidate == id)?;
        Some(self.slots.remove(position).1)
    }

    pub(crate) fn take(&mut self) -> Vec<Waker> {
        self.slots.drain(..).map(|(_, waker)| waker).collect()
    }
}

pub(crate) trait Condition {
    type State;

    fn lock(&self) -> MutexGuard<'_, Self::State>;

    fn holds(&self, state: &Self::State) -> bool;

    fn waiters(state: &mut Self::State) -> &mut Waiters;
}

pub(crate) struct Wait<C: Condition> {
    condition: C,
    slot: Option<u64>,
}

impl<C: Condition> Wait<C> {
    pub(crate) fn new(condition: C) -> Self {
        Self {
            condition,
            slot: None,
        }
    }
}

impl<C: Condition + Unpin> Future for Wait<C> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
        let this = &mut *self;
        let waker = context.waker().clone();
        let (poll, displaced) = {
            let mut state = this.condition.lock();
            if this.condition.holds(&state) {
                (
                    Poll::Ready(()),
                    C::waiters(&mut state).cancel(this.slot.take()),
                )
            } else {
                (
                    Poll::Pending,
                    C::waiters(&mut state).register(&mut this.slot, waker),
                )
            }
        };
        drop(displaced);
        poll
    }
}

impl<C: Condition> Drop for Wait<C> {
    fn drop(&mut self) {
        let removed = {
            let mut state = self.condition.lock();
            C::waiters(&mut state).cancel(self.slot)
        };
        drop(removed);
    }
}
