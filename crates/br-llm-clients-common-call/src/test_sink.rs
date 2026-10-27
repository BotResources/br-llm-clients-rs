use std::sync::Arc;
use std::time::Duration;

use br_llm_clients_core::{BoxFuture, StreamSink};
use br_llm_messages::ResponseDelta;
use tokio::time::Instant;

use crate::test_decoder::DecoderLog;

#[derive(Default)]
pub(crate) struct RecordingSink {
    pub(crate) deltas: Vec<ResponseDelta>,
    pub(crate) payloads_seen: Vec<usize>,
    pub(crate) at: Vec<Instant>,
    pub(crate) log: Option<Arc<DecoderLog>>,
    pub(crate) pause: Option<Duration>,
}

impl RecordingSink {
    pub(crate) fn watching(log: &Arc<DecoderLog>) -> Self {
        Self {
            log: Some(Arc::clone(log)),
            ..Self::default()
        }
    }
}

impl StreamSink for RecordingSink {
    fn send<'a>(&'a mut self, delta: ResponseDelta) -> BoxFuture<'a, ()> {
        let seen = self.log.as_ref().map(|log| log.payload_count());
        self.payloads_seen.extend(seen);
        self.at.push(Instant::now());
        self.deltas.push(delta);
        let pause = self.pause;
        Box::pin(async move {
            if let Some(pause) = pause {
                tokio::time::advance(pause).await;
            }
        })
    }
}
