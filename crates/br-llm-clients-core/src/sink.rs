use br_llm_messages::StreamEvent;

use crate::client::BoxFuture;

pub trait StreamSink: Send {
    fn send<'a>(&'a mut self, event: StreamEvent) -> BoxFuture<'a, ()>;
}
