use br_llm_clients_common_call::ErrorInfo;
use br_llm_clients_core::ClientError;
use serde_json::Value;

const QUOTA_CODES: [&str; 5] = [
    "insufficient_quota",
    "credit_balance_exhausted",
    "organization_spend_limit_exceeded",
    "project_spend_limit_exceeded",
    "organization_usage_limit_exceeded",
];

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
    let code = info.code()?;
    QUOTA_CODES
        .contains(&code)
        .then_some(ClientError::QuotaExceeded)
}
