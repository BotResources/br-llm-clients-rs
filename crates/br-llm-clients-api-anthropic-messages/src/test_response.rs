use br_llm_messages::{ContentBlock, ModelResponse, ReplayMaterial};
use serde_json::Value;

pub(crate) fn texts(response: &ModelResponse) -> Vec<String> {
    response
        .blocks()
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text(text) => Some(text.text().as_str().to_owned()),
            _ => None,
        })
        .collect()
}

pub(crate) struct Call {
    pub(crate) id: Option<String>,
    pub(crate) name: String,
    pub(crate) arguments: Value,
}

pub(crate) fn tool_calls(response: &ModelResponse) -> Vec<Call> {
    response
        .runtime_calls()
        .map(|call| Call {
            id: call.external_id().map(|id| id.as_str().to_owned()),
            name: call.name().as_str().to_owned(),
            arguments: call.arguments().deserialize_into().expect("json arguments"),
        })
        .collect()
}

pub(crate) fn thinking(response: &ModelResponse) -> Vec<(String, Option<ReplayMaterial>)> {
    response
        .blocks()
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Thinking(thinking) => {
                Some((thinking.text().to_owned(), thinking.replay().cloned()))
            }
            _ => None,
        })
        .collect()
}

pub(crate) fn structured(response: &ModelResponse) -> Vec<Value> {
    response
        .blocks()
        .iter()
        .filter_map(|block| match block {
            ContentBlock::StructuredContent(structured) => {
                Some(structured.value().deserialize_into().expect("a json value"))
            }
            _ => None,
        })
        .collect()
}
