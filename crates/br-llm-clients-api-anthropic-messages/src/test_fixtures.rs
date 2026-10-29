use br_llm_clients_common_call::Dialect;
use br_llm_clients_common_call::EncodedRequest;
use br_llm_clients_core::{
    CacheRequirement, ClientError, ClientRequest, ClientSettings, MaxOutputTokens, OutputMode,
    Parameter, ParameterKind, Parameters, ReasoningLevel, StructuredShape, ToolSpec,
    render_history,
};
use br_llm_messages::{ModelName, PlainText, ToolName, Turn};
use serde_json::{Value, json};

pub(crate) use crate::test_thread::user;
use crate::test_thread::{project, reply_entry, reply_turn};
use crate::{AnthropicMessages, MessagesOptions, VersionPlacement};

pub(crate) const MODEL: &str = "claude-test-model";

pub(crate) fn settings(
    level: ReasoningLevel,
    cache: CacheRequirement,
    max_tokens: u32,
) -> ClientSettings {
    ClientSettings::new(
        ModelName::new(MODEL).expect("model"),
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

pub(crate) fn request(turns: Vec<Turn>) -> ClientRequest {
    request_with(None, turns, Vec::new(), OutputMode::Plain)
}

pub(crate) fn request_with(
    system: Option<&str>,
    turns: Vec<Turn>,
    tools: Vec<ToolSpec>,
    output: OutputMode,
) -> ClientRequest {
    ClientRequest::new(
        system.map(|value| PlainText::new(value).expect("a system prompt")),
        project(turns),
        reply_turn(),
        reply_entry(),
        tools,
        output,
    )
    .expect("request")
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
    let name = ToolName::new(name).expect("tool name");
    ToolSpec::new(name, description, parameters(list)).expect("tool")
}

pub(crate) fn structured(description: Option<&str>, list: Vec<Parameter>) -> OutputMode {
    OutputMode::Structured(
        StructuredShape::new("answer", description.map(str::to_owned), parameters(list))
            .expect("shape"),
    )
}

pub(crate) fn encode(
    dialect: &AnthropicMessages,
    request: &ClientRequest,
) -> Result<EncodedRequest, ClientError> {
    let history = render_history(request).expect("a rendering");
    dialect.encode(request, &history)
}

pub(crate) fn body_of(dialect: &AnthropicMessages, request: &ClientRequest) -> Value {
    let encoded = encode(dialect, request).expect("encoded");
    serde_json::from_slice(encoded.body()).expect("json body")
}

pub(crate) fn refusal_of(dialect: &AnthropicMessages, request: &ClientRequest) -> ClientError {
    encode(dialect, request).expect_err("a refusal")
}

pub(crate) fn simple_body() -> Value {
    body_of(&dialect(), &request(vec![user("hello")]))
}

pub(crate) fn expected_simple_messages() -> Value {
    json!([{"role": "user", "content": [{"type": "text", "text": "hello"}]}])
}

pub(crate) fn vertex() -> VersionPlacement {
    VersionPlacement::Body("vertex-2023-10-16".to_owned())
}
