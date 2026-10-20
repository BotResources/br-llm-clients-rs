use br_llm_clients_common_call::Dialect;
use br_llm_clients_core::{
    CacheRequirement, ClientError, ClientRequest, ClientSettings, MaxOutputTokens, OutputMode,
    Parameter, ParameterKind, Parameters, ReasoningLevel, StructuredShape, ToolSpec,
};
use br_llm_messages::{
    AssistantBlock, Base64Data, ImageMime, ModelId, RedactedThinking, Signature, Text, Thinking,
    ToolCall, ToolCallId, ToolName, ToolResult, ToolResultBlock, WireMessage, WireUserBlock,
};
use serde_json::{Value, json};

use crate::{AnthropicMessages, MessagesOptions};

pub(crate) const MODEL: &str = "claude-test-model";

pub(crate) fn settings(
    level: ReasoningLevel,
    cache: CacheRequirement,
    max_tokens: u32,
) -> ClientSettings {
    ClientSettings::new(
        ModelId::new(MODEL).expect("model"),
        level,
        cache,
        MaxOutputTokens::new(max_tokens).expect("max tokens"),
    )
}

pub(crate) fn dialect() -> AnthropicMessages {
    dialect_with(ReasoningLevel::Off, CacheRequirement::NotRequired)
}

pub(crate) fn dialect_with(level: ReasoningLevel, cache: CacheRequirement) -> AnthropicMessages {
    AnthropicMessages::new(&settings(level, cache, 1024))
}

pub(crate) fn dialect_options(options: MessagesOptions) -> AnthropicMessages {
    AnthropicMessages::with_options(
        &settings(ReasoningLevel::Off, CacheRequirement::NotRequired, 1024),
        options,
    )
}

pub(crate) fn text(value: &str) -> Text {
    Text::new(value).expect("text")
}

pub(crate) fn user(value: &str) -> WireMessage {
    WireMessage::User {
        content: vec![WireUserBlock::Text { text: text(value) }],
    }
}

pub(crate) fn assistant(blocks: Vec<AssistantBlock>) -> WireMessage {
    WireMessage::Assistant { content: blocks }
}

pub(crate) fn assistant_text(value: &str) -> WireMessage {
    assistant(vec![AssistantBlock::Text { text: text(value) }])
}

pub(crate) fn relay(texts: &[&str]) -> WireMessage {
    WireMessage::Relay {
        content: texts.iter().map(|value| text(value)).collect(),
    }
}

pub(crate) fn tool_name(value: &str) -> ToolName {
    ToolName::new(value).expect("tool name")
}

pub(crate) fn call_id(value: &str) -> ToolCallId {
    ToolCallId::new(value).expect("tool call id")
}

pub(crate) fn tool_call(id: &str, name: &str, arguments: Value) -> AssistantBlock {
    AssistantBlock::ToolCall(ToolCall {
        id: call_id(id),
        name: tool_name(name),
        arguments,
    })
}

pub(crate) fn thinking(value: &str, signature: Option<&str>) -> AssistantBlock {
    AssistantBlock::Thinking(Thinking {
        text: value.to_owned(),
        signature: signature.map(|signature| Signature::new(signature).expect("signature")),
    })
}

pub(crate) fn redacted(data: &str) -> AssistantBlock {
    AssistantBlock::RedactedThinking(RedactedThinking::new(data).expect("redacted"))
}

pub(crate) fn tool_result(id: &str, name: &str, texts: &[&str], is_error: bool) -> WireUserBlock {
    WireUserBlock::ToolResult(ToolResult::new(
        call_id(id),
        tool_name(name),
        texts
            .iter()
            .map(|value| ToolResultBlock::text(text(value)))
            .collect(),
        is_error,
    ))
}

pub(crate) fn user_blocks(blocks: Vec<WireUserBlock>) -> WireMessage {
    WireMessage::User { content: blocks }
}

pub(crate) fn user_text_block(value: &str) -> WireUserBlock {
    WireUserBlock::Text { text: text(value) }
}

pub(crate) fn image(mime: ImageMime, data: &str) -> WireUserBlock {
    WireUserBlock::Image(br_llm_messages::Image {
        mime,
        data: Base64Data::new(data).expect("base64"),
    })
}

pub(crate) fn request(messages: Vec<WireMessage>) -> ClientRequest {
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

pub(crate) fn param(
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

pub(crate) fn structured(description: Option<&str>, list: Vec<Parameter>) -> OutputMode {
    OutputMode::Structured(
        StructuredShape::new("answer", description.map(str::to_owned), parameters(list))
            .expect("shape"),
    )
}

pub(crate) fn body_of(dialect: &AnthropicMessages, request: &ClientRequest) -> Value {
    let encoded = dialect.encode(request).expect("encoded");
    serde_json::from_slice(encoded.body()).expect("json body")
}

pub(crate) fn refusal_of(dialect: &AnthropicMessages, request: &ClientRequest) -> ClientError {
    dialect.encode(request).expect_err("a refusal")
}

pub(crate) fn simple_body() -> Value {
    body_of(&dialect(), &request(vec![user("hello")]))
}

pub(crate) fn expected_simple_messages() -> Value {
    json!([{"role": "user", "content": [{"type": "text", "text": "hello"}]}])
}
