use br_llm_messages::ResponseDelta;

use crate::client::BoxFuture;

pub trait StreamSink: Send {
    fn send<'a>(&'a mut self, delta: ResponseDelta) -> BoxFuture<'a, ()>;
}
