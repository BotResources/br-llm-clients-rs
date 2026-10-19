use br_llm_clients_core::{BoxFuture, StreamSink};
use br_llm_messages::StreamEvent;

#[derive(Default)]
pub(crate) struct RecordingSink {
    pub(crate) events: Vec<StreamEvent>,
}

impl StreamSink for RecordingSink {
    fn send<'a>(&'a mut self, event: StreamEvent) -> BoxFuture<'a, ()> {
        self.events.push(event);
        Box::pin(async {})
    }
}
