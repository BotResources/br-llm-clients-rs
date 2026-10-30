use br_llm_clients_common_call::{Dialect, EncodedRequest};
use br_llm_clients_core::{
    CacheRequirement, ClientError, ClientRequest, ClientSettings, MaxOutputTokens, OutputMode,
    Parameter, ParameterKind, Parameters, ReasoningLevel, RenderedHistory, StructuredShape,
    ToolSpec, render_history,
};
use br_llm_messages::{ModelName, PlainText, ToolName, Turn};
use serde_json::{Value, json};

use crate::test_history::{project, reply_entry, reply_turn, say};
use crate::{OpenAiResponses, Options};

pub(crate) fn tool_name(value: &str) -> ToolName {
    ToolName::new(value).expect("tool name")
}

pub(crate) fn user_text(value: &str) -> Turn {
    say(value)
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
    let system = system.map(|text| PlainText::new(text).expect("system"));
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

pub(crate) fn rendered(request: &ClientRequest) -> RenderedHistory {
    render_history(request).expect("a rendering")
}

pub(crate) fn settings(level: ReasoningLevel, cache: CacheRequirement) -> ClientSettings {
    ClientSettings::new(
        ModelName::new("gpt-5.1").expect("model"),
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

pub(crate) fn encoded(
    dialect: &OpenAiResponses,
    request: &ClientRequest,
) -> Result<EncodedRequest, ClientError> {
    dialect.encode(request, &rendered(request))
}

pub(crate) fn encode_body(
    dialect: &OpenAiResponses,
    request: &ClientRequest,
) -> Result<Value, ClientError> {
    let encoded = dialect.encode(request, &rendered(request))?;
    Ok(serde_json::from_slice(encoded.body()).expect("the body is JSON"))
}

pub(crate) fn body_of(request: &ClientRequest) -> Value {
    encode_body(&dialect(ReasoningLevel::Off), request).expect("encoded")
}

pub(crate) fn input_of(turns: Vec<Turn>) -> Vec<Value> {
    body_of(&request(turns))["input"]
        .as_array()
        .expect("input list")
        .clone()
}

pub(crate) fn refusal_of(turns: Vec<Turn>) -> ClientError {
    encode_body(&dialect(ReasoningLevel::Off), &request(turns)).expect_err("a refusal")
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
