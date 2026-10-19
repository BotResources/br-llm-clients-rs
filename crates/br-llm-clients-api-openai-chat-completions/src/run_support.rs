use std::sync::Arc;
use std::time::Duration;

use br_llm_clients_common_call::{CallSpec, Dialect, run};
use br_llm_clients_common_http::{HttpClient, HttpConfig, NoAuthorizer};
use br_llm_clients_core::{CallOutcome, ClientError, ClientRequest, OutputMode};
use br_llm_messages::{Step, StreamEvent};

use crate::ChatCompletionsDialect;
use crate::test_server::{TestServer, Wire, http};
use crate::test_sink::RecordingSink;
use crate::test_support::{dialect, hello};

const LONG: Duration = Duration::from_secs(3600);

pub(crate) fn sse(chunks: &[String]) -> Vec<u8> {
    chunks
        .iter()
        .flat_map(|data| format!("data: {data}\n\n").into_bytes())
        .collect()
}

pub(crate) fn sse_with_sentinel(chunks: &[String]) -> Vec<u8> {
    let mut bytes = sse(chunks);
    bytes.extend_from_slice(b"data: [DONE]\n\n");
    bytes
}

pub(crate) fn ok_stream(bytes: &[u8]) -> Wire {
    http("200 OK", &[("content-type", "text/event-stream")], bytes)
}

pub(crate) fn spec_for(dialect: ChatCompletionsDialect, server: &TestServer) -> CallSpec {
    let dialect: Arc<dyn Dialect> = Arc::new(dialect);
    CallSpec::new(
        dialect,
        server.endpoint("/v1/chat/completions"),
        Arc::new(NoAuthorizer),
        HttpClient::new(HttpConfig::new(LONG, LONG, LONG)).expect("client"),
        br_llm_messages::ModelId::new("gpt-test").expect("model"),
    )
}

pub(crate) struct Called {
    pub(crate) outcome: CallOutcome,
    pub(crate) events: Vec<StreamEvent>,
    pub(crate) server: TestServer,
}

impl Called {
    pub(crate) fn step(&self) -> &Step {
        self.outcome.result().expect("a step")
    }

    pub(crate) fn error(&self) -> &ClientError {
        self.outcome.result().expect_err("an error")
    }
}

pub(crate) async fn call_with(
    dialect: ChatCompletionsDialect,
    request: &ClientRequest,
    reply: impl Fn(usize) -> Wire + Send + Sync + 'static,
) -> Called {
    let server = TestServer::start(reply).await;
    let spec = spec_for(dialect, &server);
    let mut sink = RecordingSink::default();
    let outcome = run(&spec, request, &mut sink).await;
    Called {
        outcome,
        events: sink.events,
        server,
    }
}

pub(crate) async fn call_stream(chunks: &[String]) -> Called {
    let bytes = sse_with_sentinel(chunks);
    call_with(dialect(), &hello(), move |_| ok_stream(&bytes)).await
}

pub(crate) async fn call_structured_stream(chunks: &[String], output: OutputMode) -> Called {
    let bytes = sse_with_sentinel(chunks);
    let request = crate::test_support::request_with(
        None,
        vec![crate::test_support::user_text("go")],
        Vec::new(),
        output,
    );
    call_with(dialect(), &request, move |_| ok_stream(&bytes)).await
}

pub(crate) async fn call_refused(status: &str, headers: &[(&str, &str)], body: &str) -> Called {
    let status = status.to_owned();
    let headers: Vec<(String, String)> = headers
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect();
    let body = body.as_bytes().to_vec();
    call_with(dialect(), &hello(), move |_| {
        let pairs: Vec<(&str, &str)> = headers
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect();
        http(&status, &pairs, &body)
    })
    .await
}
