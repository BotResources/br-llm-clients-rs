use br_llm_clients_common_call::{Dialect, Payload, PayloadDecoder, PayloadError, ProviderEvent};
use br_llm_clients_core::{
    CacheRequirement, ClientError, ClientRequest, ClientSettings, MaxOutputTokens, OutputMode,
    Parameter, ParameterKind, Parameters, ReasoningLevel, StructuredShape, ToolSpec,
};
use br_llm_messages::{
    AssistantBlock, Base64Data, ImageMime, ModelId, Text, ToolCall, ToolCallId, ToolName,
    ToolResult, ToolResultBlock, WireMessage, WireUserBlock,
};
use serde_json::Value;

use crate::{ChatCompletionsDialect, ChatCompletionsOptions, ReasoningTable};

pub(crate) fn settings(level: ReasoningLevel) -> ClientSettings {
    ClientSettings::new(
        ModelId::new("gpt-test").expect("model"),
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

pub(crate) fn text(value: &str) -> Text {
    Text::new(value).expect("text")
}

pub(crate) fn call_id(value: &str) -> ToolCallId {
    ToolCallId::new(value).expect("call id")
}

pub(crate) fn tool_name(value: &str) -> ToolName {
    ToolName::new(value).expect("tool name")
}

pub(crate) fn user_text(value: &str) -> WireMessage {
    WireMessage::User {
        content: vec![WireUserBlock::Text { text: text(value) }],
    }
}

pub(crate) fn user(content: Vec<WireUserBlock>) -> WireMessage {
    WireMessage::User { content }
}

pub(crate) fn image(mime: ImageMime, data: &str) -> WireUserBlock {
    WireUserBlock::Image(br_llm_messages::Image {
        mime,
        data: Base64Data::new(data).expect("data"),
    })
}

pub(crate) fn tool_result(id: &str, name: &str, texts: &[&str], is_error: bool) -> WireUserBlock {
    let content = texts
        .iter()
        .map(|value| ToolResultBlock::text(text(value)))
        .collect();
    WireUserBlock::ToolResult(ToolResult::new(
        call_id(id),
        tool_name(name),
        content,
        is_error,
    ))
}

pub(crate) fn assistant(content: Vec<AssistantBlock>) -> WireMessage {
    WireMessage::Assistant { content }
}

pub(crate) fn assistant_text(value: &str) -> AssistantBlock {
    AssistantBlock::Text { text: text(value) }
}

pub(crate) fn assistant_call(id: &str, name: &str, arguments: Value) -> AssistantBlock {
    AssistantBlock::ToolCall(ToolCall {
        id: call_id(id),
        name: tool_name(name),
        arguments,
    })
}

pub(crate) fn request_of(messages: Vec<WireMessage>) -> ClientRequest {
    request_with(None, messages, Vec::new(), OutputMode::Plain)
}

pub(crate) fn request_with(
    system: Option<&str>,
    messages: Vec<WireMessage>,
    tools: Vec<ToolSpec>,
    output: OutputMode,
) -> ClientRequest {
    ClientRequest::new(system.map(text), messages, tools, output).expect("request")
}

pub(crate) fn hello() -> ClientRequest {
    request_of(vec![user_text("hello")])
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

pub(crate) fn encoded_body(dialect: &ChatCompletionsDialect, request: &ClientRequest) -> Value {
    let encoded = dialect.encode(request).expect("encoded");
    serde_json::from_slice(encoded.body()).expect("json body")
}

pub(crate) fn encode_error(
    dialect: &ChatCompletionsDialect,
    request: &ClientRequest,
) -> ClientError {
    dialect.encode(request).expect_err("a refusal")
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
