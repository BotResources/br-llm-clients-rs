use std::sync::Arc;
use std::time::Duration;

use br_llm_clients_common_call::{CallSpec, run};
use br_llm_clients_common_http::{HttpClient, HttpConfig, NoAuthorizer};
use br_llm_clients_core::{
    CacheRequirement, CallOutcome, ClientError, ClientRequest, OutputMode, ReasoningLevel,
};
use br_llm_messages::{ModelId, Step, StreamEvent};

use crate::AnthropicMessages;
use crate::test_fixtures::{MODEL, dialect_with, request, request_with, structured, user};
use crate::test_server::{Received, TestServer, Wire, http};
use crate::test_sink::RecordingSink;

const LONG: Duration = Duration::from_secs(3600);

pub(crate) struct Called {
    pub(crate) outcome: CallOutcome,
    pub(crate) events: Vec<StreamEvent>,
    pub(crate) received: Vec<Received>,
}

impl Called {
    pub(crate) fn step(&self) -> &Step {
        self.outcome.result().expect("a step")
    }

    pub(crate) fn error(&self) -> &ClientError {
        self.outcome.result().expect_err("an error")
    }
}

pub(crate) fn plain_dialect() -> AnthropicMessages {
    dialect_with(ReasoningLevel::Off, CacheRequirement::NotRequired)
}

pub(crate) fn hello() -> ClientRequest {
    request(vec![user("hello")])
}

pub(crate) fn structured_hello() -> ClientRequest {
    request_with(None, vec![user("hello")], vec![], structured(None, vec![]))
}

pub(crate) fn plain_output() -> OutputMode {
    OutputMode::Plain
}

pub(crate) fn spec(dialect: AnthropicMessages, server: &TestServer) -> CallSpec {
    CallSpec::new(
        Arc::new(dialect),
        server.endpoint("/v1/messages"),
        Arc::new(NoAuthorizer),
        HttpClient::new(HttpConfig::new(LONG, LONG, LONG)).expect("client"),
        ModelId::new(MODEL).expect("model"),
    )
}

pub(crate) async fn call_with(
    dialect: AnthropicMessages,
    request: &ClientRequest,
    reply: impl Fn(usize) -> Wire + Send + Sync + 'static,
) -> Called {
    let server = TestServer::start(reply).await;
    let spec = spec(dialect, &server);
    let mut sink = RecordingSink::default();
    let outcome = run(&spec, request, &mut sink).await;
    Called {
        outcome,
        events: sink.events,
        received: server.requests(),
    }
}

pub(crate) async fn call_stream(request: &ClientRequest, body: &str) -> Called {
    let bytes = body.as_bytes().to_vec();
    call_with(plain_dialect(), request, move |_| {
        http("200 OK", &[("content-type", "text/event-stream")], &bytes)
    })
    .await
}

pub(crate) async fn call_refusal(
    status: &str,
    headers: &[(&str, &str)],
    body: &str,
) -> ClientError {
    let owned: Vec<(String, String)> = headers
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect();
    let (status, body) = (status.to_owned(), body.to_owned());
    let called = call_with(plain_dialect(), &hello(), move |_| {
        let headers: Vec<(&str, &str)> = owned
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect();
        http(&status, &headers, body.as_bytes())
    })
    .await;
    called.error().clone()
}
