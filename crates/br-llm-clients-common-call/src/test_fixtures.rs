use std::sync::Arc;
use std::time::Duration;

use br_llm_clients_common_http::{Endpoint, HttpClient, HttpConfig, NoAuthorizer};
use br_llm_clients_core::{CallOutcome, ClientError, ClientRequest, OutputMode, StreamSink};
use br_llm_messages::{ModelId, StreamEvent, Text, WireMessage, WireUserBlock};

use crate::call_spec::CallSpec;
use crate::test_dialect::ScriptedDialect;

pub(crate) const LONG: Duration = Duration::from_secs(3600);

pub(crate) fn http_client() -> HttpClient {
    HttpClient::new(HttpConfig::new(LONG, LONG, LONG)).expect("client")
}

pub(crate) fn model() -> ModelId {
    ModelId::new("test-model").expect("model")
}

pub(crate) fn user_request() -> ClientRequest {
    let text = Text::new("hello").expect("text");
    ClientRequest::new(
        None,
        vec![WireMessage::User {
            content: vec![WireUserBlock::Text { text }],
        }],
        Vec::new(),
        OutputMode::Plain,
    )
    .expect("request")
}

pub(crate) fn assistant_first_request() -> ClientRequest {
    let text = Text::new("hello").expect("text");
    ClientRequest::new(
        None,
        vec![WireMessage::Assistant {
            content: vec![br_llm_messages::AssistantBlock::Text { text }],
        }],
        Vec::new(),
        OutputMode::Plain,
    )
    .expect("request")
}

pub(crate) fn spec_for(dialect: &Arc<ScriptedDialect>, endpoint: Endpoint) -> CallSpec {
    CallSpec::new(
        dialect.clone(),
        endpoint,
        Arc::new(NoAuthorizer),
        http_client(),
        model(),
    )
}

pub(crate) struct Discard;

impl StreamSink for Discard {
    fn send<'a>(&'a mut self, _event: StreamEvent) -> br_llm_clients_core::BoxFuture<'a, ()> {
        Box::pin(async {})
    }
}

pub(crate) async fn run_call(spec: &CallSpec, request: &ClientRequest) -> CallOutcome {
    crate::run::run(spec, request, &mut Discard).await
}

pub(crate) async fn failure_of(spec: &CallSpec, request: &ClientRequest) -> ClientError {
    run_call(spec, request)
        .await
        .into_parts()
        .1
        .expect_err("a failure")
}
