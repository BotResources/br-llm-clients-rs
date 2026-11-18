use std::time::Duration;

use br_llm_clients_common_http::{Endpoint, HttpClient, HttpConfig, Secret};
use br_llm_clients_core::{
    BoxFuture, CacheRequirement, Capabilities, ClientRequest, ClientSettings, MaxOutputTokens,
    OutputMode, ReasoningLevel, StreamSink,
};
use br_llm_messages::AgentId;
use br_llm_messages::{
    BlockId, Conversation, ConversationId, EntryId, HumanId, HumanMessage, IncomingEntry,
    InputTurn, ModelName, Perspective, PlainBlock, ResponseDelta, TextBlock, Turn, TurnId, Uuid,
};

use crate::api_family::ApiFamily;
use crate::client::LiteLlmClient;
use crate::config::LiteLlmConfig;
use crate::{ChatCompletionsOptions, ReasoningTable};

pub(crate) const KEY: &str = "sk-test-key-0123456789";

pub(crate) fn client(base: Endpoint, extra_headers: Vec<(String, String)>) -> LiteLlmClient {
    let settings = ClientSettings::new(
        ModelName::new("test-group").unwrap(),
        ReasoningLevel::Off,
        CacheRequirement::NotRequired,
        MaxOutputTokens::new(64).unwrap(),
    );
    let family =
        ApiFamily::ChatCompletions(ChatCompletionsOptions::new(ReasoningTable::openai_effort()));
    let config = LiteLlmConfig::new(
        base,
        Secret::new(KEY).unwrap(),
        family,
        Capabilities::everything(),
    )
    .with_extra_headers(extra_headers);
    let http = HttpClient::new(HttpConfig::new(
        Duration::from_secs(5),
        Duration::from_secs(5),
        Duration::from_secs(5),
    ))
    .unwrap();
    LiteLlmClient::new(settings, config, http).unwrap()
}

pub(crate) fn hello() -> ClientRequest {
    let human = HumanId::from_uuid(Uuid::now_v7()).unwrap();
    let text = PlainBlock::Text(TextBlock::new(BlockId::mint(), "Hello").unwrap());
    let message = HumanMessage::new(EntryId::mint(), human, vec![text]).unwrap();
    let input = InputTurn::new(TurnId::mint(), vec![IncomingEntry::from(message)]).unwrap();
    let conversation =
        Conversation::from_parts(ConversationId::mint(), vec![Turn::InputTurn(input)]).unwrap();
    let agent = AgentId::from_uuid(Uuid::now_v7()).unwrap();
    let history = Perspective::of(agent).project(&conversation);
    ClientRequest::new(
        None,
        history,
        TurnId::mint(),
        EntryId::mint(),
        Vec::new(),
        OutputMode::Plain,
    )
    .unwrap()
}

pub(crate) fn answer_stream() -> Vec<u8> {
    [
        r#"{"model":"test-group","choices":[{"delta":{"content":"Hi"}}]}"#,
        r#"{"choices":[{"delta":{},"finish_reason":"stop"}]}"#,
        r#"{"choices":[],"usage":{"prompt_tokens":1,"completion_tokens":2}}"#,
    ]
    .iter()
    .flat_map(|data| format!("data: {data}\n\n").into_bytes())
    .chain(b"data: [DONE]\n\n".iter().copied())
    .collect()
}

#[derive(Default)]
pub(crate) struct NullSink;

impl StreamSink for NullSink {
    fn send<'a>(&'a mut self, _delta: ResponseDelta) -> BoxFuture<'a, ()> {
        Box::pin(async {})
    }
}
