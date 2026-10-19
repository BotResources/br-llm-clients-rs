use serde_json::Value;

pub(crate) struct ReasoningEnvelope {
    pub(crate) id: String,
    pub(crate) encrypted_content: String,
}

pub(crate) fn make(id: &str, encrypted_content: &str) -> String {
    serde_json::json!({ "id": id, "encrypted_content": encrypted_content }).to_string()
}

pub(crate) fn parse(signature: &str) -> Option<ReasoningEnvelope> {
    let Value::Object(fields) = serde_json::from_str::<Value>(signature).ok()? else {
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
