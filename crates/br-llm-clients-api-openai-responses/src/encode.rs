use br_llm_clients_common_call::EncodedRequest;
use br_llm_clients_core::{ClientError, ClientRequest, OutputMode, ReasoningLevel};
use serde_json::{Map, Value};

use crate::input::items;
use crate::options::Options;
use crate::reasoning_plan::plan;
use crate::structured::text_field;
use crate::tools::tools;

pub(crate) struct Fixed<'a> {
    pub(crate) model: &'a str,
    pub(crate) reasoning: ReasoningLevel,
    pub(crate) max_output_tokens: u32,
    pub(crate) options: &'a Options,
}

pub(crate) fn encode(
    fixed: &Fixed<'_>,
    request: &ClientRequest,
) -> Result<EncodedRequest, ClientError> {
    let reasoning = plan(fixed.reasoning, fixed.options)?;
    let input = items(request.messages())?;
    let mut body = Map::new();
    body.insert("model".to_owned(), Value::String(fixed.model.to_owned()));
    body.insert("stream".to_owned(), Value::Bool(true));
    body.insert("store".to_owned(), Value::Bool(false));
    if let Some(system) = request.system() {
        body.insert(
            "instructions".to_owned(),
            Value::String(system.as_str().to_owned()),
        );
    }
    body.insert("input".to_owned(), Value::Array(input));
    body.insert(
        "max_output_tokens".to_owned(),
        Value::from(fixed.max_output_tokens),
    );
    if !request.tools().is_empty() {
        body.insert("tools".to_owned(), Value::Array(tools(request.tools())?));
    }
    if let OutputMode::Structured(shape) = request.output() {
        body.insert("text".to_owned(), text_field(shape)?);
    }
    if let Some(reasoning) = reasoning.reasoning {
        body.insert("reasoning".to_owned(), reasoning);
    }
    if let Some(include) = reasoning.include {
        body.insert("include".to_owned(), include);
    }
    Ok(EncodedRequest::new(Value::Object(body).to_string())
        .with_header("Content-Type", "application/json"))
}
