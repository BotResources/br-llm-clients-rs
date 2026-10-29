use std::sync::Arc;
use std::time::Duration;

use br_llm_clients_common_http::{Endpoint, HttpClient, HttpConfig, NoAuthorizer};
use br_llm_clients_core::{
    CallOutcome, ClientError, ClientRequest, IdSource, OutputMode, StreamSink,
};
use br_llm_messages::{
    AITurn, AgentId, Author, ContentBlock, Conversation, ConversationId, EntryId, HumanId,
    HumanMessage, IncomingEntry, InputTurn, ModelResponse, OpaqueJson, Perspective, PlainBlock,
    ProjectedThread, ResponseDelta, ResponseEnding, ResponseMetadata, RuntimeToolCall, TextBlock,
    ToolName, Turn, TurnEntry, TurnId,
};

use crate::call_spec::CallSpec;
use crate::test_dialect::ScriptedDialect;
use crate::test_ids::{CountingIds, block_of, reply_entry, reply_turn, uuid_of};

pub(crate) const LONG: Duration = Duration::from_secs(3600);

pub(crate) fn http_client() -> HttpClient {
    HttpClient::new(HttpConfig::new(LONG, LONG, LONG)).expect("client")
}

fn agent() -> AgentId {
    AgentId::from_uuid(uuid_of(0x3000)).expect("agent")
}

fn conversation() -> ConversationId {
    ConversationId::from_uuid(uuid_of(0x3200)).expect("conversation")
}

fn greeting() -> Turn {
    let content = vec![PlainBlock::Text(
        TextBlock::new(block_of(0x4000), "hello").expect("text"),
    )];
    let human = HumanId::from_uuid(uuid_of(0x3100)).expect("human");
    let entry = EntryId::from_uuid(uuid_of(0x3400)).expect("entry");
    let message = HumanMessage::new(entry, human, content).expect("message");
    let turn = TurnId::from_uuid(uuid_of(0x3300)).expect("turn");
    let entries = vec![IncomingEntry::from(message)];
    Turn::InputTurn(InputTurn::new(turn, entries).expect("input turn"))
}

fn project(turns: Vec<Turn>) -> ProjectedThread {
    let conversation = Conversation::from_parts(conversation(), turns).expect("conversation");
    Perspective::of(agent()).project(&conversation)
}

pub(crate) fn request_of(
    thread: ProjectedThread,
    tools: Vec<br_llm_clients_core::ToolSpec>,
    output: OutputMode,
) -> ClientRequest {
    ClientRequest::new(None, thread, reply_turn(), reply_entry(), tools, output).expect("request")
}

pub(crate) fn user_request() -> ClientRequest {
    request_of(project(vec![greeting()]), Vec::new(), OutputMode::Plain)
}

pub(crate) fn request_with(
    tools: Vec<br_llm_clients_core::ToolSpec>,
    output: OutputMode,
) -> ClientRequest {
    request_of(project(vec![greeting()]), tools, output)
}

pub(crate) fn rendered_of(request: &ClientRequest) -> br_llm_clients_core::RenderedHistory {
    br_llm_clients_core::render_history(request).expect("a rendering")
}

pub(crate) fn unrenderable_request() -> ClientRequest {
    let call = RuntimeToolCall::new(
        block_of(0x4100),
        ToolName::new("search").expect("name"),
        OpaqueJson::parse("{}").expect("json"),
    )
    .expect("call");
    let response = ModelResponse::new(
        EntryId::from_uuid(uuid_of(0x3500)).expect("entry"),
        vec![ContentBlock::RuntimeToolCall(call)],
        ResponseEnding::tool_use(),
        ResponseMetadata::new(),
    )
    .expect("response");
    let turn = AITurn::from_parts(
        TurnId::from_uuid(uuid_of(0x3600)).expect("turn"),
        agent(),
        Author::new("target").expect("author"),
        vec![TurnEntry::ModelResponse(response)],
    )
    .expect("ai turn");
    request_of(
        project(vec![greeting(), Turn::AITurn(turn)]),
        Vec::new(),
        OutputMode::Plain,
    )
}

pub(crate) fn spec_with_ids(
    dialect: &Arc<ScriptedDialect>,
    endpoint: Endpoint,
    ids: Arc<dyn IdSource>,
) -> CallSpec {
    spec_for(dialect, endpoint).with_id_source(ids)
}

pub(crate) fn spec_for(dialect: &Arc<ScriptedDialect>, endpoint: Endpoint) -> CallSpec {
    CallSpec::new(
        dialect.clone(),
        endpoint,
        Arc::new(NoAuthorizer),
        http_client(),
    )
    .with_id_source(Arc::new(CountingIds::new()))
}

pub(crate) struct Discard;

impl StreamSink for Discard {
    fn send<'a>(&'a mut self, _delta: ResponseDelta) -> br_llm_clients_core::BoxFuture<'a, ()> {
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

pub(crate) struct Busy(tokio::task::JoinHandle<()>);

impl Busy {
    pub(crate) fn keep() -> Self {
        Self(tokio::spawn(async {
            loop {
                tokio::task::yield_now().await;
            }
        }))
    }
}

impl Drop for Busy {
    fn drop(&mut self) {
        self.0.abort();
    }
}
