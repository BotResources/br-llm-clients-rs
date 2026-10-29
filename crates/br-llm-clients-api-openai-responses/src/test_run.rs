use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use br_llm_clients_common_call::{CallSpec, Endpoint, run};
use br_llm_clients_common_http::{HttpClient, HttpConfig, NoAuthorizer};
use br_llm_clients_core::{
    BoxFuture, CallOutcome, ClientError, ClientRequest, IdSource, ReasoningLevel, StreamSink,
};
use br_llm_messages::{
    BlockId, ContentBlock, ModelResponse, ResponseDelta, RuntimeToolCall, ThinkingBlock,
};
use serde_json::Value;

use crate::OpenAiResponses;
use crate::test_history::uuid_of;
use crate::test_requests::dialect;
use crate::test_server::{Received, TestServer, Wire, http};

const LONG: Duration = Duration::from_secs(3600);

pub(crate) struct CountingIds(AtomicU64);

impl IdSource for CountingIds {
    fn next_block_id(&self) -> BlockId {
        let n = self.0.fetch_add(1, Ordering::SeqCst);
        BlockId::from_uuid(uuid_of(0x7000_0000 | n)).expect("block id")
    }
}

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

pub(crate) fn sse(events: &[Value]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for event in events {
        let name = event["type"].as_str().expect("type");
        bytes.extend_from_slice(format!("event: {name}\ndata: {event}\n\n").as_bytes());
    }
    bytes
}

pub(crate) fn ok_wire(body: &[u8]) -> Wire {
    http("200 OK", &[("content-type", "text/event-stream")], body)
}

pub(crate) fn spec_with(dialect: OpenAiResponses, endpoint: Endpoint) -> CallSpec {
    CallSpec::new(
        Arc::new(dialect),
        endpoint,
        Arc::new(NoAuthorizer),
        HttpClient::new(HttpConfig::new(LONG, LONG, LONG)).expect("client"),
    )
    .with_id_source(Arc::new(CountingIds(AtomicU64::new(1))))
}

pub(crate) struct Called {
    pub(crate) outcome: CallOutcome,
    pub(crate) sink: RecordingSink,
    pub(crate) requests: Vec<Received>,
}

impl Called {
    pub(crate) fn reply(self) -> ModelResponse {
        self.outcome.into_parts().1.expect("a model response")
    }

    pub(crate) fn failure(self) -> ClientError {
        self.outcome.into_parts().1.expect_err("a failure")
    }
}

pub(crate) async fn call_with_wire(
    dialect: OpenAiResponses,
    request: &ClientRequest,
    reply: impl Fn(usize) -> Wire + Send + Sync + 'static,
) -> Called {
    let server = TestServer::start(reply).await;
    let spec = spec_with(dialect, server.endpoint("/v1/responses"));
    let mut sink = RecordingSink::default();
    let outcome = run(&spec, request, &mut sink).await;
    Called {
        outcome,
        sink,
        requests: server.requests(),
    }
}

pub(crate) async fn call(request: &ClientRequest, wire: Wire) -> Called {
    call_with_wire(dialect(ReasoningLevel::Off), request, move |_| wire.clone()).await
}

pub(crate) async fn call_stream(request: &ClientRequest, events: &[Value]) -> Called {
    call(request, ok_wire(&sse(events))).await
}

pub(crate) fn texts(reply: &ModelResponse) -> Vec<String> {
    reply
        .blocks()
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text(text) => Some(text.text().as_str().to_owned()),
            _ => None,
        })
        .collect()
}

pub(crate) fn calls(reply: &ModelResponse) -> Vec<RuntimeToolCall> {
    reply.runtime_calls().cloned().collect()
}

pub(crate) fn thinkings(reply: &ModelResponse) -> Vec<ThinkingBlock> {
    reply
        .blocks()
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Thinking(thinking) => Some(thinking.clone()),
            _ => None,
        })
        .collect()
}

pub(crate) fn kinds(reply: &ModelResponse) -> Vec<&'static str> {
    reply
        .blocks()
        .iter()
        .map(|block| match block {
            ContentBlock::Text(_) => "text",
            ContentBlock::Thinking(_) => "thinking",
            ContentBlock::RuntimeToolCall(_) => "call",
            ContentBlock::StructuredContent(_) => "structured",
            _ => "other",
        })
        .collect()
}
