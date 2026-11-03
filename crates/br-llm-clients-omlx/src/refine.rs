use br_llm_clients_common_call::ErrorInfo;
use br_llm_clients_core::ClientError;
use serde_json::Value;

const NOT_FOUND_CODE: &str = "not_found_error";
const CONTEXT_CODE: &str = "context_length_exceeded";
const LIMIT_START: &str = "> ";
const LIMIT_END: &str = " maximum context length";

pub(crate) fn refine(info: &ErrorInfo) -> Option<ClientError> {
    match (info.status()?, info.code()) {
        (404, Some(NOT_FOUND_CODE)) => Some(ClientError::ModelNotFound),
        (400, Some(CONTEXT_CODE)) => Some(ClientError::ContextTooLong {
            limit: context_limit(info.text()),
        }),
        (409, _) => Some(ClientError::Unavailable { retry_after: None }),
        _ => None,
    }
}

fn context_limit(body: &str) -> Option<u64> {
    let root = serde_json::from_str::<Value>(body).ok()?;
    let message = root.get("error")?.get("message")?.as_str()?;
    let (_, tail) = message.rsplit_once(LIMIT_START)?;
    tail.strip_suffix(LIMIT_END)?.parse::<u64>().ok()
}
