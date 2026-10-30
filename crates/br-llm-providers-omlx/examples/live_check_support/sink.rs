use br_llm_clients_core::{BoxFuture, StreamSink};
use br_llm_messages::ResponseDelta;

#[derive(Default)]
pub struct CountingSink {
    pub deltas: usize,
}

impl StreamSink for CountingSink {
    fn send<'a>(&'a mut self, _delta: ResponseDelta) -> BoxFuture<'a, ()> {
        self.deltas = self.deltas.saturating_add(1);
        Box::pin(async {})
    }
}
