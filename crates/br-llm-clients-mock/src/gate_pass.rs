use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use crate::call_log::CallState;
use crate::gate::{Gate, GateState, Ticket};
use crate::sync::{lock, wake_all};
use crate::tracker::CallTracker;

pub(crate) struct GatePass {
    gate: Gate,
    tracker: CallTracker,
    ticket: Option<u64>,
}

struct Advance {
    poll: Poll<()>,
    spare: Option<Waker>,
    wakers: Vec<Waker>,
}

impl Advance {
    fn ready(spare: Waker) -> Self {
        Self {
            poll: Poll::Ready(()),
            spare: Some(spare),
            wakers: Vec::new(),
        }
    }

    fn pending(spare: Option<Waker>, wakers: Vec<Waker>) -> Self {
        Self {
            poll: Poll::Pending,
            spare,
            wakers,
        }
    }
}

impl GatePass {
    pub(crate) fn new(gate: Gate, tracker: CallTracker) -> Self {
        Self {
            gate,
            tracker,
            ticket: None,
        }
    }
}

fn arrive(
    ticket: &mut Option<u64>,
    tracker: &CallTracker,
    state: &mut GateState,
    waker: Waker,
) -> Advance {
    let id = state.next_ticket;
    state.next_ticket += 1;
    let mut wakers = tracker.set(CallState::WaitingAtGate);
    state.queue.push_back(Ticket {
        id,
        tracker: tracker.clone(),
        waker,
    });
    wakers.extend(state.counters.take());
    *ticket = Some(id);
    Advance::pending(None, wakers)
}

fn advance(
    ticket: &mut Option<u64>,
    tracker: &CallTracker,
    state: &mut GateState,
    waker: Waker,
) -> Advance {
    if state.open {
        return Advance::ready(waker);
    }
    let Some(id) = *ticket else {
        return arrive(ticket, tracker, state, waker);
    };
    if let Some(position) = state.released.iter().position(|released| *released == id) {
        state.released.swap_remove(position);
        *ticket = None;
        return Advance::ready(waker);
    }
    let spare = match state.queue.iter_mut().find(|queued| queued.id == id) {
        Some(queued) if !queued.waker.will_wake(&waker) => {
            Some(std::mem::replace(&mut queued.waker, waker))
        }
        _ => Some(waker),
    };
    Advance::pending(spare, Vec::new())
}

impl Future for GatePass {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
        let this = &mut *self;
        let waker = context.waker().clone();
        let advanced = {
            let mut state = lock(&this.gate.state);
            advance(&mut this.ticket, &this.tracker, &mut state, waker)
        };
        drop(advanced.spare);
        wake_all(advanced.wakers);
        advanced.poll
    }
}

impl Drop for GatePass {
    fn drop(&mut self) {
        let Some(id) = self.ticket else {
            return;
        };
        let (removed, wakers) = {
            let mut state = lock(&self.gate.state);
            let position = state.queue.iter().position(|queued| queued.id == id);
            let removed = position.and_then(|position| state.queue.remove(position));
            let mut wakers = Vec::new();
            if removed.is_some() {
                wakers.extend(self.tracker.set(CallState::Dropped));
                wakers.extend(state.counters.take());
            }
            state.released.retain(|released| *released != id);
            (removed, wakers)
        };
        drop(removed);
        wake_all(wakers);
    }
}
