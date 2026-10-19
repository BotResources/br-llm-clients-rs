use std::sync::Arc;
use std::time::Duration;

use br_llm_clients_common_call::{CallSpec, Endpoint, run};
use br_llm_clients_common_http::{HttpClient, HttpConfig, NoAuthorizer};
use br_llm_clients_core::{BoxFuture, CallOutcome, ClientRequest, ReasoningLevel, StreamSink};
use br_llm_messages::{ModelId, StreamEvent};
use serde_json::Value;

use crate::OpenAiResponses;
use crate::test_requests::dialect;
use crate::test_server::{Received, TestServer, Wire, http};

const LONG: Duration = Duration::from_secs(3600);

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

pub(crate) fn spec_with(dialect: OpenAiResponses, endpoint: Endpoint, model: &str) -> CallSpec {
    CallSpec::new(
        Arc::new(dialect),
        endpoint,
        Arc::new(NoAuthorizer),
        HttpClient::new(HttpConfig::new(LONG, LONG, LONG)).expect("client"),
        ModelId::new(model).expect("model"),
    )
}

pub(crate) struct Called {
    pub(crate) outcome: CallOutcome,
    pub(crate) sink: RecordingSink,
    pub(crate) requests: Vec<Received>,
}

pub(crate) async fn call_with_wire(
    dialect: OpenAiResponses,
    request: &ClientRequest,
    reply: impl Fn(usize) -> Wire + Send + Sync + 'static,
) -> Called {
    let server = TestServer::start(reply).await;
    let spec = spec_with(dialect, server.endpoint("/v1/responses"), "gpt-5.1");
    let mut sink = RecordingSink::default();
    let outcome = run(&spec, request, &mut sink).await;
    Called {
        outcome,
        sink,
        requests: server.requests(),
    }
}

pub(crate) async fn call(request: &ClientRequest, wire: Wire) -> Called {
    let bytes = wire.bytes;
    call_with_wire(dialect(ReasoningLevel::Off), request, move |_| Wire {
        bytes: bytes.clone(),
    })
    .await
}

pub(crate) async fn call_stream(request: &ClientRequest, events: &[Value]) -> Called {
    call(request, ok_wire(&sse(events))).await
}
