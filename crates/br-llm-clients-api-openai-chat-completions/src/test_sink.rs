use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use br_llm_clients_core::{BoxFuture, StreamSink};
use br_llm_messages::StreamEvent;

#[derive(Default)]
pub(crate) struct RecordingSink {
    pub(crate) events: Vec<StreamEvent>,
    pub(crate) seen: Arc<AtomicUsize>,
}

impl RecordingSink {
    pub(crate) fn counting(seen: &Arc<AtomicUsize>) -> Self {
        Self {
            events: Vec::new(),
            seen: Arc::clone(seen),
        }
    }
}

impl StreamSink for RecordingSink {
    fn send<'a>(&'a mut self, event: StreamEvent) -> BoxFuture<'a, ()> {
        self.events.push(event);
        self.seen.fetch_add(1, Ordering::SeqCst);
        Box::pin(async {})
    }
}
