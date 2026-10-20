use br_llm_clients_core::{BoxFuture, StreamSink};
use br_llm_messages::StreamEvent;

#[derive(Default)]
pub struct CountingSink {
    pub events: usize,
}

impl StreamSink for CountingSink {
    fn send<'a>(&'a mut self, _event: StreamEvent) -> BoxFuture<'a, ()> {
        self.events += 1;
        Box::pin(async {})
    }
}
