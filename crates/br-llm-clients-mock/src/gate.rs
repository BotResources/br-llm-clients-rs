use std::collections::VecDeque;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use crate::call_log::CallState;
use crate::sync::{Waiters, lock, wake_all};
use crate::tracker::CallTracker;

pub(crate) struct Ticket {
    id: u64,
    tracker: CallTracker,
    waker: Waker,
}

#[derive(Default)]
pub(crate) struct GateState {
    open: bool,
    next_ticket: u64,
    queue: VecDeque<Ticket>,
    released: Vec<u64>,
    counters: Waiters,
}

#[derive(Clone, Default)]
pub struct Gate {
    state: Arc<Mutex<GateState>>,
}

impl Gate {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&self) {
        let mut wakers = Vec::new();
        {
            let mut state = lock(&self.state);
            state.open = true;
            for ticket in state.queue.drain(..) {
                wakers.extend(ticket.tracker.set(CallState::Running));
                wakers.push(ticket.waker);
            }
            wakers.extend(state.counters.take());
        }
        wake_all(wakers);
    }

    pub fn release(&self, count: usize) {
        let mut wakers = Vec::new();
        {
            let mut state = lock(&self.state);
            for _ in 0..count {
                let Some(ticket) = state.queue.pop_front() else {
                    break;
                };
                wakers.extend(ticket.tracker.set(CallState::Running));
                wakers.push(ticket.waker);
                state.released.push(ticket.id);
            }
            wakers.extend(state.counters.take());
        }
        wake_all(wakers);
    }

    pub fn waiting(&self) -> usize {
        lock(&self.state).queue.len()
    }

    pub fn is_open(&self) -> bool {
        lock(&self.state).open
    }

    pub fn wait_for_waiting(&self, count: usize) -> WaitForWaiting {
        WaitForWaiting {
            gate: self.clone(),
            count,
            slot: None,
        }
    }

    pub(crate) fn pass(&self, tracker: CallTracker) -> GatePass {
        GatePass {
            gate: self.clone(),
            tracker,
            ticket: None,
        }
    }
}

impl fmt::Debug for Gate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = lock(&self.state);
        formatter
            .debug_struct("Gate")
            .field("open", &state.open)
            .field("waiting", &state.queue.len())
            .finish()
    }
}

pub struct WaitForWaiting {
    gate: Gate,
    count: usize,
    slot: Option<u64>,
}

impl Future for WaitForWaiting {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
        let this = &mut *self;
        let mut state = lock(&this.gate.state);
        if state.queue.len() >= this.count {
            return Poll::Ready(());
        }
        state.counters.register(&mut this.slot, context.waker());
        Poll::Pending
    }
}

impl Drop for WaitForWaiting {
    fn drop(&mut self) {
        lock(&self.gate.state).counters.cancel(self.slot);
    }
}

pub(crate) struct GatePass {
    gate: Gate,
    tracker: CallTracker,
    ticket: Option<u64>,
}

impl GatePass {
    fn arrive(&mut self, state: &mut GateState, waker: &Waker) -> Vec<Waker> {
        let id = state.next_ticket;
        state.next_ticket += 1;
        let mut wakers = self.tracker.set(CallState::WaitingAtGate);
        state.queue.push_back(Ticket {
            id,
            tracker: self.tracker.clone(),
            waker: waker.clone(),
        });
        wakers.extend(state.counters.take());
        self.ticket = Some(id);
        wakers
    }
}

impl Future for GatePass {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
        let this = &mut *self;
        let gate = this.gate.clone();
        let mut state = lock(&gate.state);
        if state.open {
            return Poll::Ready(());
        }
        let Some(id) = this.ticket else {
            let wakers = this.arrive(&mut state, context.waker());
            drop(state);
            wake_all(wakers);
            return Poll::Pending;
        };
        if let Some(position) = state.released.iter().position(|released| *released == id) {
            state.released.swap_remove(position);
            this.ticket = None;
            return Poll::Ready(());
        }
        if let Some(ticket) = state.queue.iter_mut().find(|ticket| ticket.id == id)
            && !ticket.waker.will_wake(context.waker())
        {
            ticket.waker = context.waker().clone();
        }
        Poll::Pending
    }
}

impl Drop for GatePass {
    fn drop(&mut self) {
        let Some(id) = self.ticket else {
            return;
        };
        let mut wakers = Vec::new();
        {
            let mut state = lock(&self.gate.state);
            if let Some(position) = state.queue.iter().position(|ticket| ticket.id == id) {
                state.queue.remove(position);
                wakers.extend(self.tracker.set(CallState::Dropped));
                wakers.extend(state.counters.take());
            }
            state.released.retain(|released| *released != id);
        }
        wake_all(wakers);
    }
}
