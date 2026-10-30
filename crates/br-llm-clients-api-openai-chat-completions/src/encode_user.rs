use br_llm_clients_core::{ClientError, RenderedPart, RenderedResultPart};
use br_llm_messages::{ImageBlock, ImageSource, ToolName};
use serde_json::{Map, Value};

use crate::encode_parts::{invalid, message, text_part};
use crate::encode_plan::{UserPart, UserPlan};
use crate::options::ChatCompletionsOptions;

pub(crate) fn plan_user(
    parts: &[RenderedPart],
    options: &ChatCompletionsOptions,
) -> Result<UserPlan, ClientError> {
    let mut plan = UserPlan::default();
    for part in parts {
        match part {
            RenderedPart::Text(text) => plan.parts.push(UserPart::Text(text.clone())),
            RenderedPart::Image(image) => plan.parts.push(UserPart::Image(image_part(image)?)),
            RenderedPart::ToolResult {
                wire_id,
                tool_name,
                parts,
                ..
            } => plan
                .tools
                .push(tool_message(wire_id, tool_name, parts, options)?),
            RenderedPart::ToolCall { .. } => {
                return Err(invalid(
                    "a user message holds a tool call, which this dialect cannot send",
                ));
            }
            RenderedPart::Block(block) => {
                return Err(invalid(format!(
                    "a user message holds a block of kind {}, which this dialect cannot send",
                    block.kind().as_str()
                )));
            }
            _ => {
                return Err(invalid(
                    "a user message holds a part of a kind that this dialect does not know",
                ));
            }
        }
    }
    Ok(plan)
}

pub(crate) fn user_messages(plan: UserPlan) -> Vec<Value> {
    let mut messages = plan.tools;
    if !plan.parts.is_empty() {
        messages.push(user_message(plan.parts));
    }
    messages
}

fn tool_message(
    wire_id: &str,
    tool_name: &ToolName,
    parts: &[RenderedResultPart],
    options: &ChatCompletionsOptions,
) -> Result<Value, ClientError> {
    let mut texts = Vec::new();
    for part in parts {
        match part {
            RenderedResultPart::Text(text) => texts.push(text.as_str()),
            RenderedResultPart::Image(_) => {
                return Err(invalid(
                    "the result of a tool holds an image, which this dialect cannot send",
                ));
            }
            _ => {
                return Err(invalid(
                    "the result of a tool holds a part of a kind that this dialect does not know",
                ));
            }
        }
    }
    let content = match texts.as_slice() {
        [] => Value::from(options.empty_tool_result_text().unwrap_or("")),
        [only] => Value::from(*only),
        many => Value::Array(many.iter().map(|text| text_part(text)).collect()),
    };
    let mut tool = message("tool");
    tool.insert("tool_call_id".to_owned(), Value::from(wire_id));
    tool.insert("content".to_owned(), content);
    if options.tool_message_name() {
        tool.insert("name".to_owned(), Value::from(tool_name.as_str()));
    }
    Ok(Value::Object(tool))
}

fn user_message(parts: Vec<UserPart>) -> Value {
    let content = match parts.as_slice() {
        [UserPart::Text(text)] => Value::from(text.as_str()),
        _ => Value::Array(
            parts
                .into_iter()
                .map(|part| match part {
                    UserPart::Text(text) => text_part(&text),
                    UserPart::Image(image) => image,
                })
                .collect(),
        ),
    };
    let mut user = message("user");
    user.insert("content".to_owned(), content);
    Value::Object(user)
}

fn image_part(image: &ImageBlock) -> Result<Value, ClientError> {
    let (media_type, data) = match image.source() {
        ImageSource::Inline {
            media_type, data, ..
        } => (media_type.as_str(), data.as_str()),
        _ => {
            return Err(invalid(
                "an image has a source of a kind that this dialect does not know",
            ));
        }
    };
    let mut url = Map::new();
    url.insert(
        "url".to_owned(),
        Value::from(format!("data:{media_type};base64,{data}")),
    );
    let mut part = Map::new();
    part.insert("type".to_owned(), Value::from("image_url"));
    part.insert("image_url".to_owned(), Value::Object(url));
    Ok(Value::Object(part))
}
