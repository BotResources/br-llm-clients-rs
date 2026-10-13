use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Context, Poll};

use br_llm_clients_core::{BoxFuture, StreamSink};
use br_llm_messages::{ModelId, Step, StepDraft, StreamEvent};

use crate::fold_error::FoldError;
use crate::sync::{Condition, Wait, Waiters, lock, wake_all};

#[derive(Default)]
struct Recorded {
    events: Vec<StreamEvent>,
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

    pub fn events(&self) -> Vec<StreamEvent> {
        lock(&self.recorded).events.clone()
    }

    pub fn take(&self) -> Vec<StreamEvent> {
        std::mem::take(&mut lock(&self.recorded).events)
    }

    pub fn len(&self) -> usize {
        lock(&self.recorded).events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn wait_for_events(&self, count: usize) -> WaitForEvents {
        WaitForEvents(Wait::new(EventsHeld {
            sink: self.clone(),
            count,
        }))
    }

    pub fn fold(&self, model: Option<ModelId>) -> Result<Step, FoldError> {
        let events = self.events();
        let mut draft = StepDraft::new();
        for event in events {
            draft
                .apply(event)
                .map_err(|error| FoldError::new(error.to_string()))?;
        }
        draft
            .finish(model)
            .map_err(|error| FoldError::new(error.to_string()))
    }

    fn record(&self, event: StreamEvent) {
        let wakers = {
            let mut recorded = lock(&self.recorded);
            recorded.events.push(event);
            recorded.waiters.take()
        };
        wake_all(wakers);
    }
}

impl std::fmt::Debug for RecordingSink {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RecordingSink")
            .field("events", &self.len())
            .finish()
    }
}

impl StreamSink for RecordingSink {
    fn send<'a>(&'a mut self, event: StreamEvent) -> BoxFuture<'a, ()> {
        Box::pin(async move { self.record(event) })
    }
}

struct EventsHeld {
    sink: RecordingSink,
    count: usize,
}

impl Condition for EventsHeld {
    type State = Recorded;

    fn lock(&self) -> MutexGuard<'_, Recorded> {
        lock(&self.sink.recorded)
    }

    fn holds(&self, state: &Recorded) -> bool {
        state.events.len() >= self.count
    }

    fn waiters(state: &mut Recorded) -> &mut Waiters {
        &mut state.waiters
    }
}

#[must_use = "futures do nothing unless awaited"]
pub struct WaitForEvents(Wait<EventsHeld>);

impl Future for WaitForEvents {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
        Pin::new(&mut self.0).poll(context)
    }
}
