use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, MutexGuard};
use std::task::{Context, Poll};

use crate::call_log::StateKind;
use crate::shared::{Shared, State};
use crate::sync::{Condition, Wait, Waiters};

struct CallsBegun {
    shared: Arc<Shared>,
    count: usize,
}

impl Condition for CallsBegun {
    type State = State;

    fn lock(&self) -> MutexGuard<'_, State> {
        self.shared.guard()
    }

    fn holds(&self, state: &State) -> bool {
        state.log.began() >= self.count
    }

    fn waiters(state: &mut State) -> &mut Waiters {
        state.log.waiters_mut()
    }
}

struct CallsInState {
    shared: Arc<Shared>,
    kind: StateKind,
    count: usize,
}

impl Condition for CallsInState {
    type State = State;

    fn lock(&self) -> MutexGuard<'_, State> {
        self.shared.guard()
    }

    fn holds(&self, state: &State) -> bool {
        state.log.count_in(self.kind) >= self.count
    }

    fn waiters(state: &mut State) -> &mut Waiters {
        state.log.waiters_mut()
    }
}

#[must_use = "futures do nothing unless awaited"]
pub struct WaitForCalls(Wait<CallsBegun>);

impl WaitForCalls {
    pub(crate) fn new(shared: Arc<Shared>, count: usize) -> Self {
        Self(Wait::new(CallsBegun { shared, count }))
    }
}

impl Future for WaitForCalls {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
        Pin::new(&mut self.0).poll(context)
    }
}

#[must_use = "futures do nothing unless awaited"]
pub struct WaitForState(Wait<CallsInState>);

impl WaitForState {
    pub(crate) fn new(shared: Arc<Shared>, kind: StateKind, count: usize) -> Self {
        Self(Wait::new(CallsInState {
            shared,
            kind,
            count,
        }))
    }
}

impl Future for WaitForState {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
        Pin::new(&mut self.0).poll(context)
    }
}
