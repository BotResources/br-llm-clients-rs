use br_llm_clients_common_call::ErrorInfo;
use br_llm_clients_core::ClientError;
use serde_json::Value;

const QUOTA_CODE: &str = "insufficient_quota";

pub(crate) fn error_code(body: &str) -> Option<String> {
    let Ok(Value::Object(root)) = serde_json::from_str::<Value>(body) else {
        return None;
    };
    code_of_error(root.get("error")?)
}

pub(crate) fn code_of_error(error: &Value) -> Option<String> {
    let object = error.as_object()?;
    object
        .get("code")
        .and_then(Value::as_str)
        .or_else(|| object.get("type").and_then(Value::as_str))
        .map(str::to_owned)
}

pub(crate) fn refine_error(info: &ErrorInfo) -> Option<ClientError> {
    match info.code() {
        Some(QUOTA_CODE) => Some(ClientError::QuotaExceeded),
        _ => None,
    }
}
