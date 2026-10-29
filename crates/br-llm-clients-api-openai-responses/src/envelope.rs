use br_llm_messages::{ReplayFormat, ReplayMaterial, ReplayPayload};
use serde_json::Value;

pub(crate) const REPLAY_FORMAT: &str = "br-llm-clients-api-openai-responses/reasoning";

pub(crate) struct ReasoningEnvelope {
    pub(crate) id: String,
    pub(crate) encrypted_content: String,
}

pub(crate) fn replay_format() -> Option<ReplayFormat> {
    ReplayFormat::new(REPLAY_FORMAT).ok()
}

pub(crate) fn make(id: &str, encrypted_content: &str) -> String {
    serde_json::json!({ "id": id, "encrypted_content": encrypted_content }).to_string()
}

pub(crate) fn of_material(material: &ReplayMaterial) -> Option<ReasoningEnvelope> {
    if material.format().as_str() != REPLAY_FORMAT {
        return None;
    }
    match material.payload() {
        ReplayPayload::Text { value, .. } => parse(value),
        _ => None,
    }
}

fn parse(text: &str) -> Option<ReasoningEnvelope> {
    let Value::Object(fields) = serde_json::from_str::<Value>(text).ok()? else {
        return None;
    };
    if fields.len() != 2 {
        return None;
    }
    let id = non_empty_text(fields.get("id")?)?;
    let encrypted_content = non_empty_text(fields.get("encrypted_content")?)?;
    Some(ReasoningEnvelope {
        id,
        encrypted_content,
    })
}

fn non_empty_text(value: &Value) -> Option<String> {
    value
        .as_str()
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}
