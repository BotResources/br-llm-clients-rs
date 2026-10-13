use std::collections::VecDeque;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Context, Poll, Waker};

use crate::call_log::CallState;
use crate::gate_pass::GatePass;
use crate::sync::{Condition, Wait, Waiters, lock, wake_all};
use crate::tracker::CallTracker;

pub(crate) struct Ticket {
    pub(crate) id: u64,
    pub(crate) tracker: CallTracker,
    pub(crate) waker: Waker,
}

#[derive(Default)]
pub(crate) struct GateState {
    pub(crate) open: bool,
    pub(crate) next_ticket: u64,
    pub(crate) queue: VecDeque<Ticket>,
    pub(crate) released: Vec<u64>,
    pub(crate) counters: Waiters,
}

#[derive(Clone, Default)]
pub struct Gate {
    pub(crate) state: Arc<Mutex<GateState>>,
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
        WaitForWaiting(Wait::new(GateWaiting {
            gate: self.clone(),
            count,
        }))
    }

    pub(crate) fn pass(&self, tracker: CallTracker) -> GatePass {
        GatePass::new(self.clone(), tracker)
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

struct GateWaiting {
    gate: Gate,
    count: usize,
}

impl Condition for GateWaiting {
    type State = GateState;

    fn lock(&self) -> MutexGuard<'_, GateState> {
        lock(&self.gate.state)
    }

    fn holds(&self, state: &GateState) -> bool {
        state.queue.len() >= self.count
    }

    fn waiters(state: &mut GateState) -> &mut Waiters {
        &mut state.counters
    }
}

#[must_use = "futures do nothing unless awaited"]
pub struct WaitForWaiting(Wait<GateWaiting>);

impl Future for WaitForWaiting {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
        Pin::new(&mut self.0).poll(context)
    }
}
