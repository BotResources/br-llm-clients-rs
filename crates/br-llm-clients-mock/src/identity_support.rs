use br_llm_messages::{ContentBlock, ModelResponse, PlainBlock, Uuid};
use serde_json::Value;

fn text_ids(text: &br_llm_messages::TextBlock, out: &mut Vec<Uuid>) {
    out.push(text.id().as_uuid());
    for citation in text.citations() {
        out.push(citation.id().as_uuid());
    }
}

fn plain_ids(blocks: &[PlainBlock], out: &mut Vec<Uuid>) {
    for block in blocks {
        match block {
            PlainBlock::Text(text) => text_ids(text, out),
            PlainBlock::Image(image) => out.push(image.id().as_uuid()),
            PlainBlock::StructuredContent(structured) => out.push(structured.id().as_uuid()),
            _ => {}
        }
    }
}

pub(crate) fn ids_in_order(response: &ModelResponse) -> Vec<Uuid> {
    let mut out = Vec::new();
    for block in response.blocks() {
        match block {
            ContentBlock::Text(text) => text_ids(text, &mut out),
            ContentBlock::ProviderToolResult(result) => {
                out.push(result.id().as_uuid());
                plain_ids(result.content(), &mut out);
            }
            ContentBlock::ProviderToolExecutionRecord(record) => {
                out.push(record.id().as_uuid());
                plain_ids(record.content(), &mut out);
            }
            other => out.push(other.id().as_uuid()),
        }
    }
    out
}

fn without_block_identities(block: &mut Value) {
    let Value::Object(map) = block else {
        return;
    };
    map.remove("id");
    map.remove("call");
    if let Some(Value::Array(citations)) = map.get_mut("citations") {
        for citation in citations {
            if let Value::Object(citation) = citation {
                citation.remove("id");
            }
        }
    }
    if let Some(Value::Array(content)) = map.get_mut("content") {
        content.iter_mut().for_each(without_block_identities);
    }
}

pub(crate) fn without_identities(response: &mut Value) {
    let Value::Object(map) = response else {
        return;
    };
    map.remove("id");
    if let Some(Value::Array(content)) = map.get_mut("content") {
        content.iter_mut().for_each(without_block_identities);
    }
}

pub(crate) fn shape_of(response: &ModelResponse) -> Value {
    let mut value = serde_json::to_value(response).unwrap();
    without_identities(&mut value);
    value
}
