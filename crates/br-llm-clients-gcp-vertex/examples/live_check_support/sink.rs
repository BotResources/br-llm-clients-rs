use br_llm_clients_core::{BoxFuture, StreamSink};
use br_llm_messages::{DeltaOp, ResponseDelta};

#[derive(Default)]
pub struct CountingSink {
    deltas: usize,
    text_deltas: usize,
}

impl CountingSink {
    pub fn deltas(&self) -> usize {
        self.deltas
    }

    pub fn text_deltas(&self) -> usize {
        self.text_deltas
    }
}

impl StreamSink for CountingSink {
    fn send<'a>(&'a mut self, delta: ResponseDelta) -> BoxFuture<'a, ()> {
        self.deltas = self.deltas.saturating_add(1);
        if let DeltaOp::TextAppended { .. } = delta.op() {
            self.text_deltas = self.text_deltas.saturating_add(1);
        }
        Box::pin(async {})
    }
}
