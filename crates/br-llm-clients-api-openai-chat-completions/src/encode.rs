use br_llm_clients_common_call::EncodedRequest;
use br_llm_clients_core::{ClientError, ClientRequest, ClientSettings, RenderedHistory};
use serde_json::{Map, Value, json};

use crate::encode_messages::messages;
use crate::encode_output::response_format;
use crate::encode_tools::tools;
use crate::options::ChatCompletionsOptions;
use crate::reasoning_effort::ReasoningEffort;
use crate::reasoning_rule::ReasoningRule;

pub(crate) fn encode(
    settings: &ClientSettings,
    options: &ChatCompletionsOptions,
    request: &ClientRequest,
    history: &RenderedHistory,
) -> Result<EncodedRequest, ClientError> {
    let effort = reasoning_effort(settings, options)?;
    let messages = messages(request, history, options)?;
    let tools = tools(request.tools())?;
    let response_format = response_format(request.output())?;

    let mut body: Map<String, Value> = options.extra_body().clone();
    body.insert("model".to_owned(), Value::from(settings.model().as_str()));
    body.insert("messages".to_owned(), Value::Array(messages));
    body.insert("stream".to_owned(), Value::Bool(true));
    body.insert("stream_options".to_owned(), json!({"include_usage": true}));
    body.insert(
        options.max_tokens_field().member().to_owned(),
        Value::from(settings.max_output_tokens().get()),
    );
    if let Some(effort) = effort {
        body.insert("reasoning_effort".to_owned(), Value::from(effort.as_wire()));
    }
    if let Some(tools) = tools {
        body.insert("tools".to_owned(), tools);
    }
    if let Some(response_format) = response_format {
        body.insert("response_format".to_owned(), response_format);
    }
    let bytes =
        serde_json::to_vec(&Value::Object(body)).map_err(|_| ClientError::InvalidMessages {
            reason: "the JSON of the body cannot be written".to_owned(),
        })?;
    Ok(EncodedRequest::new(bytes).with_header("Content-Type", "application/json"))
}

fn reasoning_effort(
    settings: &ClientSettings,
    options: &ChatCompletionsOptions,
) -> Result<Option<ReasoningEffort>, ClientError> {
    let level = settings.reasoning();
    match options.reasoning_table().rule_for(level) {
        Some(ReasoningRule::Send(effort)) => Ok(Some(effort)),
        Some(ReasoningRule::Omit) => Ok(None),
        Some(ReasoningRule::Refuse) | None => Err(ClientError::ReasoningNotSupported {
            level,
            reason: "the reasoning table of this dialect refuses this level".to_owned(),
        }),
    }
}
