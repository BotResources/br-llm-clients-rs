use br_llm_clients_core::{BoxFuture, StreamSink};
use br_llm_messages::ResponseDelta;

#[derive(Default)]
pub(crate) struct RecordingSink {
    pub(crate) deltas: Vec<ResponseDelta>,
}

impl StreamSink for RecordingSink {
    fn send<'a>(&'a mut self, delta: ResponseDelta) -> BoxFuture<'a, ()> {
        self.deltas.push(delta);
        Box::pin(async {})
    }
}
