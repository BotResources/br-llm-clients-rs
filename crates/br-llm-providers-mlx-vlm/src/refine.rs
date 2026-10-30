use br_llm_clients_common_call::ErrorInfo;
use br_llm_clients_core::ClientError;
use serde_json::Value;

const BUDGET_START: &str = "Request needs ";
const LIMIT_START: &str = ", but MAX_KV_SIZE is ";
const LIMIT_END: &str = ".";

pub(crate) fn refine(info: &ErrorInfo) -> Option<ClientError> {
    if info.status()? != 400 {
        return None;
    }
    let root = serde_json::from_str::<Value>(info.text()).ok()?;
    let detail = root.get("detail")?.as_str()?;
    if !detail.starts_with(BUDGET_START) {
        return None;
    }
    let (_, tail) = detail.split_once(LIMIT_START)?;
    let limit = tail.trim_end().strip_suffix(LIMIT_END).unwrap_or(tail);
    Some(ClientError::ContextTooLong {
        limit: limit.trim().parse::<u64>().ok(),
    })
}
