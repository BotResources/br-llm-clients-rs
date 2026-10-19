use br_llm_clients_common_call::ErrorInfo;
use serde_json::Value;

use crate::error_code::code_of_error;

const FINISH_ERROR_TEXT: &str = "the provider ended the reply with an error";

pub(crate) fn error_info(error: &Value) -> ErrorInfo {
    let text = match error {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    };
    let info = ErrorInfo::new(text);
    match code_of_error(error) {
        Some(code) => info.with_code(code),
        None => info,
    }
}

pub(crate) fn finish_error_info() -> ErrorInfo {
    ErrorInfo::new(FINISH_ERROR_TEXT)
}
