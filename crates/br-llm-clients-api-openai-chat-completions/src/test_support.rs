use br_llm_clients_common_call::{
    Dialect, EncodedRequest, Payload, PayloadDecoder, PayloadError, ProviderEvent,
};
use br_llm_clients_core::{
    CacheRequirement, ClientError, ClientRequest, ClientSettings, MaxOutputTokens, OutputMode,
    Parameter, ParameterKind, Parameters, ReasoningLevel, StructuredShape, ToolSpec,
    render_history,
};
use br_llm_messages::{ModelName, PlainText, ToolName, Turn};
use serde_json::Value;

use crate::test_thread::{project, reply_entry, reply_turn, user};
use crate::{ChatCompletionsDialect, ChatCompletionsOptions, ReasoningTable};

pub(crate) fn settings(level: ReasoningLevel) -> ClientSettings {
    ClientSettings::new(
        ModelName::new("gpt-test").expect("model"),
        level,
        CacheRequirement::NotRequired,
        MaxOutputTokens::new(1024).expect("tokens"),
    )
}

pub(crate) fn options() -> ChatCompletionsOptions {
    ChatCompletionsOptions::new(ReasoningTable::openai_effort())
}

pub(crate) fn dialect_for(
    level: ReasoningLevel,
    options: ChatCompletionsOptions,
) -> ChatCompletionsDialect {
    ChatCompletionsDialect::new(settings(level), options).expect("dialect")
}

pub(crate) fn dialect() -> ChatCompletionsDialect {
    dialect_for(ReasoningLevel::Off, options())
}

pub(crate) fn tool_name(value: &str) -> ToolName {
    ToolName::new(value).expect("tool name")
}

pub(crate) fn request_of(turns: Vec<Turn>) -> ClientRequest {
    request_with(None, turns, Vec::new(), OutputMode::Plain)
}

pub(crate) fn request_with(
    system: Option<&str>,
    turns: Vec<Turn>,
    tools: Vec<ToolSpec>,
    output: OutputMode,
) -> ClientRequest {
    let system = system.map(|value| PlainText::new(value).expect("system"));
    ClientRequest::new(
        system,
        project(turns),
        reply_turn(),
        reply_entry(),
        tools,
        output,
    )
    .expect("request")
}

pub(crate) fn hello() -> ClientRequest {
    request_of(vec![user("hello")])
}

pub(crate) fn parameter(
    name: &str,
    description: &str,
    required: bool,
    kind: ParameterKind,
) -> Parameter {
    Parameter::new(name, description, required, kind).expect("parameter")
}

pub(crate) fn parameters(list: Vec<Parameter>) -> Parameters {
    Parameters::new(list).expect("parameters")
}

pub(crate) fn tool(name: &str, description: &str, list: Vec<Parameter>) -> ToolSpec {
    ToolSpec::new(tool_name(name), description, parameters(list)).expect("tool")
}

pub(crate) fn shape(name: &str, description: Option<&str>, list: Vec<Parameter>) -> OutputMode {
    OutputMode::Structured(
        StructuredShape::new(name, description.map(str::to_owned), parameters(list))
            .expect("shape"),
    )
}

pub(crate) fn encode(
    dialect: &dyn Dialect,
    request: &ClientRequest,
) -> Result<EncodedRequest, ClientError> {
    let history = render_history(request).expect("a rendering");
    dialect.encode(request, &history)
}

pub(crate) fn encoded_body(dialect: &dyn Dialect, request: &ClientRequest) -> Value {
    let encoded = encode(dialect, request).expect("encoded");
    serde_json::from_slice(encoded.body()).expect("json body")
}

pub(crate) fn messages_of(dialect: &dyn Dialect, request: &ClientRequest) -> Vec<Value> {
    encoded_body(dialect, request)["messages"]
        .as_array()
        .cloned()
        .expect("messages")
}

pub(crate) fn encode_error(dialect: &dyn Dialect, request: &ClientRequest) -> ClientError {
    encode(dialect, request).expect_err("a refusal")
}

pub(crate) fn roles(messages: &[Value]) -> Vec<&str> {
    messages
        .iter()
        .filter_map(|message| message["role"].as_str())
        .collect()
}

pub(crate) fn decoder_for(output: &OutputMode) -> Box<dyn PayloadDecoder> {
    dialect().decoder(output)
}

pub(crate) fn plain_decoder() -> Box<dyn PayloadDecoder> {
    decoder_for(&OutputMode::Plain)
}

pub(crate) fn feed(
    decoder: &mut dyn PayloadDecoder,
    data: &str,
) -> Result<Vec<ProviderEvent>, PayloadError> {
    decoder.payload(Payload::new(None, data))
}

pub(crate) fn feed_all(decoder: &mut dyn PayloadDecoder, chunks: &[String]) -> Vec<ProviderEvent> {
    chunks
        .iter()
        .flat_map(|data| feed(decoder, data).expect("a readable chunk"))
        .collect()
}

pub(crate) fn chunk(delta: Value, finish: Option<&str>) -> String {
    serde_json::json!({
        "id": "chatcmpl-123",
        "object": "chat.completion.chunk",
        "created": 1694268190,
        "model": "gpt-4o",
        "choices": [{"index": 0, "delta": delta, "logprobs": null, "finish_reason": finish}]
    })
    .to_string()
}

pub(crate) fn content_chunk(value: &str) -> String {
    chunk(serde_json::json!({"content": value}), None)
}

pub(crate) fn finish_chunk(reason: &str) -> String {
    chunk(serde_json::json!({}), Some(reason))
}

pub(crate) fn start(key: &str, block: br_llm_clients_common_call::ProviderBlock) -> ProviderEvent {
    ProviderEvent::BlockStart {
        key: key.to_owned(),
        block,
    }
}

pub(crate) fn end(key: &str) -> ProviderEvent {
    ProviderEvent::BlockEnd {
        key: key.to_owned(),
    }
}

pub(crate) fn text_delta(key: &str, value: &str) -> ProviderEvent {
    ProviderEvent::TextDelta {
        key: key.to_owned(),
        text: value.to_owned(),
    }
}

pub(crate) fn without_metadata(events: Vec<ProviderEvent>) -> Vec<ProviderEvent> {
    events
        .into_iter()
        .filter(|event| {
            !matches!(
                event,
                ProviderEvent::Model(_) | ProviderEvent::ResponseId(_)
            )
        })
        .collect()
}

pub(crate) fn run_chunks(chunks: &[String]) -> Vec<ProviderEvent> {
    let mut decoder = plain_decoder();
    without_metadata(feed_all(decoder.as_mut(), chunks))
}

pub(crate) fn fails(data: &str) -> PayloadError {
    let mut decoder = plain_decoder();
    feed(decoder.as_mut(), data).expect_err("a payload error")
}

pub(crate) fn usage_chunk(usage: Value) -> String {
    serde_json::json!({
        "id": "chatcmpl-123",
        "object": "chat.completion.chunk",
        "created": 1694268190,
        "model": "gpt-4o",
        "choices": [],
        "usage": usage
    })
    .to_string()
}

pub(crate) fn tool_chunk(fragments: Value, finish: Option<&str>) -> String {
    chunk(serde_json::json!({"tool_calls": fragments}), finish)
}
