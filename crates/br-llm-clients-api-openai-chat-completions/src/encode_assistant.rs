use br_llm_clients_core::{ClientError, RenderedPart};
use br_llm_messages::{ContentBlock, RuntimeToolCall};
use serde_json::{Map, Value};

use crate::encode_parts::{invalid, message, text_part};
use crate::encode_plan::AssistantPlan;
use crate::options::ChatCompletionsOptions;

pub(crate) fn plan_assistant(parts: &[RenderedPart]) -> Result<AssistantPlan, ClientError> {
    let mut plan = AssistantPlan::default();
    for part in parts {
        match part {
            RenderedPart::Text(text) => plan.texts.push(text.clone()),
            RenderedPart::ToolCall { call, wire_id } => plan.calls.push(tool_call(call, wire_id)?),
            RenderedPart::Block(block) => plan_block(&mut plan, block)?,
            RenderedPart::Image(_) => {
                return Err(invalid(
                    "an assistant message holds an image, which this dialect cannot send",
                ));
            }
            RenderedPart::ToolResult { .. } => {
                return Err(invalid(
                    "an assistant message holds a tool result, which this dialect cannot send",
                ));
            }
            _ => {
                return Err(invalid(
                    "an assistant message holds a part of a kind that this dialect does not know",
                ));
            }
        }
    }
    Ok(plan)
}

fn plan_block(plan: &mut AssistantPlan, block: &ContentBlock) -> Result<(), ClientError> {
    match block {
        ContentBlock::Text(text) => plan.texts.push(text.text().as_str().to_owned()),
        ContentBlock::Thinking(thinking) => {
            if thinking.replay().is_none() {
                plan.thinking.push_str(thinking.text());
            }
        }
        ContentBlock::RedactedThinking(_) => {}
        other => {
            return Err(invalid(format!(
                "an assistant message holds a block of kind {}, which this dialect cannot send",
                other.kind().as_str()
            )));
        }
    }
    Ok(())
}

pub(crate) fn assistant_message(plan: AssistantPlan, options: &ChatCompletionsOptions) -> Value {
    let mut assistant = message("assistant");
    match plan.texts.as_slice() {
        [] if !plan.calls.is_empty() && options.tool_only_content().writes_empty_text() => {
            assistant.insert("content".to_owned(), Value::from(""));
        }
        [] => {}
        [only] => {
            assistant.insert("content".to_owned(), Value::from(only.as_str()));
        }
        many => {
            let parts = many.iter().map(|text| text_part(text)).collect();
            assistant.insert("content".to_owned(), Value::Array(parts));
        }
    }
    if !plan.calls.is_empty() {
        assistant.insert("tool_calls".to_owned(), Value::Array(plan.calls));
    }
    if !plan.thinking.is_empty()
        && let Some(field) = options.reasoning_carry_back_field()
    {
        assistant.insert(field.to_owned(), Value::from(plan.thinking));
    }
    Value::Object(assistant)
}

fn tool_call(call: &RuntimeToolCall, wire_id: &str) -> Result<Value, ClientError> {
    let arguments = call
        .arguments()
        .to_json_string()
        .map_err(|_| invalid("the arguments of a tool call cannot be written as JSON"))?;
    let mut function = Map::new();
    function.insert("name".to_owned(), Value::from(call.name().as_str()));
    function.insert("arguments".to_owned(), Value::from(arguments));
    let mut encoded = Map::new();
    encoded.insert("id".to_owned(), Value::from(wire_id));
    encoded.insert("type".to_owned(), Value::from("function"));
    encoded.insert("function".to_owned(), Value::Object(function));
    Ok(Value::Object(encoded))
}
