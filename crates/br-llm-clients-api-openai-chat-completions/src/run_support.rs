use std::sync::Arc;
use std::time::Duration;

use br_llm_clients_common_call::{CallSpec, Dialect, run};
use br_llm_clients_common_http::{HttpClient, HttpConfig, NoAuthorizer};
use br_llm_clients_core::{CallOutcome, ClientError, ClientRequest, OutputMode};
use br_llm_messages::{
    ContentBlock, DeltaOp, ModelResponse, ResponseDelta, RuntimeToolCall, ThinkingBlock,
};
use serde_json::Value;

use crate::ChatCompletionsDialect;
use crate::test_server::{TestServer, Wire, http};
use crate::test_sink::RecordingSink;
use crate::test_support::{dialect, hello, request_with};
use crate::test_thread::user;

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
    )
}

pub(crate) struct Called {
    pub(crate) outcome: CallOutcome,
    pub(crate) deltas: Vec<ResponseDelta>,
    pub(crate) server: TestServer,
}

impl Called {
    pub(crate) fn response(&self) -> &ModelResponse {
        self.outcome.result().expect("a response")
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
        deltas: sink.deltas,
        server,
    }
}

pub(crate) async fn call_stream(chunks: &[String]) -> Called {
    let bytes = sse_with_sentinel(chunks);
    call_with(dialect(), &hello(), move |_| ok_stream(&bytes)).await
}

pub(crate) async fn call_structured_stream(chunks: &[String], output: OutputMode) -> Called {
    let bytes = sse_with_sentinel(chunks);
    let request = request_with(None, vec![user("go")], Vec::new(), output);
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

pub(crate) fn texts_of(response: &ModelResponse) -> Vec<&str> {
    response
        .blocks()
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text(text) => Some(text.text().as_str()),
            _ => None,
        })
        .collect()
}

pub(crate) fn thinking_of(response: &ModelResponse) -> Vec<&ThinkingBlock> {
    response
        .blocks()
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Thinking(thinking) => Some(thinking),
            _ => None,
        })
        .collect()
}

pub(crate) fn structured_of(response: &ModelResponse) -> Vec<Value> {
    response
        .blocks()
        .iter()
        .filter_map(|block| match block {
            ContentBlock::StructuredContent(structured) => Some(json_of(
                &structured.value().to_json_string().expect("json text"),
            )),
            _ => None,
        })
        .collect()
}

pub(crate) fn calls_of(response: &ModelResponse) -> Vec<&RuntimeToolCall> {
    response.runtime_calls().collect()
}

pub(crate) fn arguments_of(call: &RuntimeToolCall) -> Value {
    json_of(&call.arguments().to_json_string().expect("json text"))
}

fn json_of(text: &str) -> Value {
    serde_json::from_str(text).expect("json")
}

pub(crate) fn appended_text(deltas: &[ResponseDelta]) -> String {
    deltas
        .iter()
        .filter_map(|delta| match delta.op() {
            DeltaOp::TextAppended { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

pub(crate) struct CountingIds {
    next: std::sync::atomic::AtomicU64,
}

impl CountingIds {
    pub(crate) fn new() -> Self {
        Self {
            next: std::sync::atomic::AtomicU64::new(0x9000_0000),
        }
    }
}

impl br_llm_clients_core::IdSource for CountingIds {
    fn next_block_id(&self) -> br_llm_messages::BlockId {
        let n = self.next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        br_llm_messages::BlockId::from_uuid(crate::test_thread::uuid_of(n)).expect("a block id")
    }
}

pub(crate) fn counted_spec_for(dialect: ChatCompletionsDialect, server: &TestServer) -> CallSpec {
    spec_for(dialect, server).with_id_source(Arc::new(CountingIds::new()))
}
