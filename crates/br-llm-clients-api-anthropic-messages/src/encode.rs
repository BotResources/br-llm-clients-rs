use br_llm_clients_common_call::EncodedRequest;
use br_llm_clients_core::{ClientError, ClientRequest, OutputMode};
use serde_json::{Map, Value, json};

use crate::encode_messages::encode_messages;
pub(crate) use crate::encode_settings::EncodeSettings;
use crate::encode_settings::{cache_control, thinking};
use crate::encode_tools::{structured_format, tool_values};

const API_VERSION: &str = "2023-06-01";

pub(crate) fn encode(
    settings: &EncodeSettings,
    request: &ClientRequest,
) -> Result<EncodedRequest, ClientError> {
    let thinking = thinking(settings.reasoning())?;
    let cache = cache_control(settings.cache())?;
    if matches!(request.output(), OutputMode::Structured(_)) && !request.tools().is_empty() {
        return Err(ClientError::StructuredOutputWithToolsNotSupported);
    }
    let messages = encode_messages(request.messages())?;
    let tools = tool_values(request.tools())?;
    let format = structured_format(request.output())?;

    let mut body = Map::new();
    body.insert("model".to_owned(), json!(settings.model()));
    body.insert("max_tokens".to_owned(), json!(settings.max_tokens()));
    body.insert("stream".to_owned(), Value::Bool(true));
    body.insert("messages".to_owned(), Value::Array(messages));
    if let Some(system) = request.system() {
        body.insert("system".to_owned(), json!(system.as_str()));
    }
    if !tools.is_empty() {
        body.insert("tools".to_owned(), Value::Array(tools));
    }
    body.insert("thinking".to_owned(), thinking.field);
    let mut output_config = Map::new();
    if let Some(effort) = thinking.effort {
        output_config.insert("effort".to_owned(), json!(effort));
    }
    if let Some(format) = format {
        output_config.insert("format".to_owned(), format);
    }
    if !output_config.is_empty() {
        body.insert("output_config".to_owned(), Value::Object(output_config));
    }
    if let Some(cache) = cache {
        body.insert("cache_control".to_owned(), cache);
    }
    Ok(
        EncodedRequest::new(Value::Object(body).to_string().into_bytes())
            .with_header("content-type", "application/json")
            .with_header("anthropic-version", API_VERSION),
    )
}
