use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use br_llm_clients_core::{BoxFuture, StreamSink};
use br_llm_messages::ResponseDelta;

#[derive(Default)]
pub(crate) struct RecordingSink {
    pub(crate) deltas: Vec<ResponseDelta>,
    pub(crate) seen: Arc<AtomicUsize>,
}

impl RecordingSink {
    pub(crate) fn counting(seen: &Arc<AtomicUsize>) -> Self {
        Self {
            deltas: Vec::new(),
            seen: Arc::clone(seen),
        }
    }
}

impl StreamSink for RecordingSink {
    fn send<'a>(&'a mut self, delta: ResponseDelta) -> BoxFuture<'a, ()> {
        self.deltas.push(delta);
        self.seen.fetch_add(1, Ordering::SeqCst);
        Box::pin(async {})
    }
}
