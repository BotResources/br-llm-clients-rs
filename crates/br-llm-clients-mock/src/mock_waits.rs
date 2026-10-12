use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use crate::call_log::StateKind;
use crate::shared::Shared;

pub struct WaitForCalls {
    shared: Arc<Shared>,
    count: usize,
    slot: Option<u64>,
}

impl WaitForCalls {
    pub(crate) fn new(shared: Arc<Shared>, count: usize) -> Self {
        Self {
            shared,
            count,
            slot: None,
        }
    }
}

impl Future for WaitForCalls {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
        let this = &mut *self;
        let shared = Arc::clone(&this.shared);
        shared.with(|state| {
            if state.log.began() >= this.count {
                return Poll::Ready(());
            }
            state.log.register(&mut this.slot, context.waker());
            Poll::Pending
        })
    }
}

impl Drop for WaitForCalls {
    fn drop(&mut self) {
        let slot = self.slot;
        self.shared.with(|state| state.log.cancel(slot));
    }
}

pub struct WaitForState {
    shared: Arc<Shared>,
    kind: StateKind,
    count: usize,
    slot: Option<u64>,
}

impl WaitForState {
    pub(crate) fn new(shared: Arc<Shared>, kind: StateKind, count: usize) -> Self {
        Self {
            shared,
            kind,
            count,
            slot: None,
        }
    }
}

impl Future for WaitForState {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
        let this = &mut *self;
        let shared = Arc::clone(&this.shared);
        shared.with(|state| {
            if state.log.count_in(this.kind) >= this.count {
                return Poll::Ready(());
            }
            state.log.register(&mut this.slot, context.waker());
            Poll::Pending
        })
    }
}

impl Drop for WaitForState {
    fn drop(&mut self) {
        let slot = self.slot;
        self.shared.with(|state| state.log.cancel(slot));
    }
}
