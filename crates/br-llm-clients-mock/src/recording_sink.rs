use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Context, Poll};

use br_llm_clients_core::{BoxFuture, StreamSink};
use br_llm_messages::{ModelResponse, ResponseDelta};

use crate::fold::fold_deltas;
use crate::fold_error::FoldError;
use crate::sync::{Condition, Wait, Waiters, lock, wake_all};

#[derive(Default)]
struct Recorded {
    deltas: Vec<ResponseDelta>,
    waiters: Waiters,
}

#[derive(Clone, Default)]
pub struct RecordingSink {
    recorded: Arc<Mutex<Recorded>>,
}

impl RecordingSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn deltas(&self) -> Vec<ResponseDelta> {
        lock(&self.recorded).deltas.clone()
    }

    pub fn take(&self) -> Vec<ResponseDelta> {
        std::mem::take(&mut lock(&self.recorded).deltas)
    }

    pub fn len(&self) -> usize {
        lock(&self.recorded).deltas.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn wait_for_deltas(&self, count: usize) -> WaitForDeltas {
        WaitForDeltas(Wait::new(DeltasHeld {
            sink: self.clone(),
            count,
        }))
    }

    pub fn fold(&self) -> Result<ModelResponse, FoldError> {
        fold_deltas(&self.deltas())
    }

    fn record(&self, delta: ResponseDelta) {
        let wakers = {
            let mut recorded = lock(&self.recorded);
            recorded.deltas.push(delta);
            recorded.waiters.take()
        };
        wake_all(wakers);
    }
}

impl std::fmt::Debug for RecordingSink {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RecordingSink")
            .field("deltas", &self.len())
            .finish()
    }
}

impl StreamSink for RecordingSink {
    fn send<'a>(&'a mut self, delta: ResponseDelta) -> BoxFuture<'a, ()> {
        Box::pin(async move { self.record(delta) })
    }
}

struct DeltasHeld {
    sink: RecordingSink,
    count: usize,
}

impl Condition for DeltasHeld {
    type State = Recorded;

    fn lock(&self) -> MutexGuard<'_, Recorded> {
        lock(&self.sink.recorded)
    }

    fn holds(&self, state: &Recorded) -> bool {
        state.deltas.len() >= self.count
    }

    fn waiters(state: &mut Recorded) -> &mut Waiters {
        &mut state.waiters
    }
}

#[must_use = "futures do nothing unless awaited"]
pub struct WaitForDeltas(Wait<DeltasHeld>);

impl Future for WaitForDeltas {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
        Pin::new(&mut self.0).poll(context)
    }
}
