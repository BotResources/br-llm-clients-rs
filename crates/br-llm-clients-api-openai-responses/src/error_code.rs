use serde_json::Value;

pub(crate) fn error_code(body: &str) -> Option<String> {
    let parsed = serde_json::from_str::<Value>(body).ok()?;
    let error = parsed.get("error")?.as_object()?;
    ["code", "type"]
        .into_iter()
        .filter_map(|key| error.get(key)?.as_str())
        .find(|text| !text.is_empty())
        .map(str::to_owned)
}
