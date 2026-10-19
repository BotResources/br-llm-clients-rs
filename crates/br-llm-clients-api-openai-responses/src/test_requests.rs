use br_llm_clients_common_call::Dialect;
use br_llm_clients_core::{
    CacheRequirement, ClientError, ClientRequest, ClientSettings, MaxOutputTokens, OutputMode,
    Parameter, ParameterKind, Parameters, ReasoningLevel, StructuredShape, ToolSpec,
};
use br_llm_messages::{
    AssistantBlock, ModelId, Signature, Text, Thinking, ToolCall, ToolCallId, ToolName, ToolResult,
    ToolResultBlock, WireMessage, WireUserBlock,
};
use serde_json::{Value, json};

use crate::{OpenAiResponses, Options};

pub(crate) fn text(value: &str) -> Text {
    Text::new(value).expect("text")
}

pub(crate) fn user_text(value: &str) -> WireMessage {
    WireMessage::User {
        content: vec![WireUserBlock::Text { text: text(value) }],
    }
}

pub(crate) fn user(content: Vec<WireUserBlock>) -> WireMessage {
    WireMessage::User { content }
}

pub(crate) fn assistant(content: Vec<AssistantBlock>) -> WireMessage {
    WireMessage::Assistant { content }
}

pub(crate) fn assistant_text(value: &str) -> WireMessage {
    assistant(vec![AssistantBlock::Text { text: text(value) }])
}

pub(crate) fn tool_name(value: &str) -> ToolName {
    ToolName::new(value).expect("tool name")
}

pub(crate) fn call_block(id: &str, name: &str, arguments: Value) -> AssistantBlock {
    AssistantBlock::ToolCall(ToolCall {
        id: ToolCallId::new(id).expect("id"),
        name: tool_name(name),
        arguments,
    })
}

pub(crate) fn result_block(id: &str, name: &str, texts: &[&str], is_error: bool) -> WireUserBlock {
    let content = texts
        .iter()
        .map(|value| ToolResultBlock::text(text(value)))
        .collect();
    WireUserBlock::ToolResult(ToolResult::new(
        ToolCallId::new(id).expect("id"),
        tool_name(name),
        content,
        is_error,
    ))
}

pub(crate) fn thinking(value: &str, signature: Option<&str>) -> AssistantBlock {
    AssistantBlock::Thinking(Thinking {
        text: value.to_owned(),
        signature: signature.map(|signature| Signature::new(signature).expect("signature")),
    })
}

pub(crate) fn envelope_text(id: &str, encrypted: &str) -> String {
    format!("{{\"id\":\"{id}\",\"encrypted_content\":\"{encrypted}\"}}")
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

pub(crate) fn settings(level: ReasoningLevel, cache: CacheRequirement) -> ClientSettings {
    ClientSettings::new(
        ModelId::new("gpt-5.1").expect("model"),
        level,
        cache,
        MaxOutputTokens::new(4096).expect("tokens"),
    )
}

pub(crate) fn dialect(level: ReasoningLevel) -> OpenAiResponses {
    dialect_with(level, Options::default())
}

pub(crate) fn dialect_with(level: ReasoningLevel, options: Options) -> OpenAiResponses {
    OpenAiResponses::new(&settings(level, CacheRequirement::NotRequired), options)
}

pub(crate) fn encode_body(
    dialect: &OpenAiResponses,
    request: &ClientRequest,
) -> Result<Value, ClientError> {
    let encoded = dialect.encode(request)?;
    Ok(serde_json::from_slice(encoded.body()).expect("the body is JSON"))
}

pub(crate) fn body_of(request: &ClientRequest) -> Value {
    encode_body(&dialect(ReasoningLevel::Off), request).expect("encoded")
}

pub(crate) fn input_of(messages: Vec<WireMessage>) -> Vec<Value> {
    body_of(&request(messages))["input"]
        .as_array()
        .expect("input list")
        .clone()
}

pub(crate) fn refusal_of(messages: Vec<WireMessage>) -> ClientError {
    encode_body(&dialect(ReasoningLevel::Off), &request(messages)).expect_err("a refusal")
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

pub(crate) fn shape(
    name: &str,
    description: Option<&str>,
    list: Vec<Parameter>,
) -> StructuredShape {
    StructuredShape::new(name, description.map(str::to_owned), parameters(list)).expect("shape")
}

pub(crate) fn structured_request(shape: StructuredShape, tools: Vec<ToolSpec>) -> ClientRequest {
    request_with(
        None,
        vec![user_text("go")],
        tools,
        OutputMode::Structured(shape),
    )
}

pub(crate) fn hello() -> ClientRequest {
    request(vec![user_text("hello")])
}

pub(crate) fn expected_function_call(id: &str, name: &str, arguments: &str) -> Value {
    json!({ "type": "function_call", "call_id": id, "name": name, "arguments": arguments })
}

pub(crate) fn user_text_block(value: &str) -> WireUserBlock {
    WireUserBlock::Text { text: text(value) }
}

pub(crate) fn relay(value: &str) -> WireMessage {
    WireMessage::Relay {
        content: vec![text(value)],
    }
}
