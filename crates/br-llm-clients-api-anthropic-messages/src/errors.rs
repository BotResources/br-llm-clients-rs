use br_llm_clients_common_call::{ErrorInfo, Headers};
use br_llm_clients_core::{ClientError, LimitKind, ReasoningLevel};
use serde_json::Value;

const REQUEST_ID_HEADER: &str = "request-id";
const SPEND_LIMIT_DETAIL: &str = "enforced_spend_limit_reached";
const USAGE_LIMIT_PREFIXES: [&str; 2] = [
    "You have reached your specified API usage limits",
    "You have reached your specified workspace API usage limits",
];
const CONTEXT_PREFIX: &str = "prompt is too long";
const REASONING_PREFIXES: [&str; 3] = [
    "\"thinking.type.disabled\" is not supported",
    "To turn thinking off on this model",
    "adaptive thinking is not supported on this model",
];

pub(crate) fn error_code(body: &str) -> Option<String> {
    let parsed = serde_json::from_str::<Value>(body).ok()?;
    parsed
        .get("error")?
        .as_object()?
        .get("type")?
        .as_str()
        .map(str::to_owned)
}

pub(crate) fn request_id(headers: &Headers) -> Option<String> {
    headers
        .get_text(REQUEST_ID_HEADER)
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
}

pub(crate) fn refine_error(reasoning: ReasoningLevel, info: &ErrorInfo) -> Option<ClientError> {
    let parsed = serde_json::from_str::<Value>(info.text()).ok();
    let by_text = match (info.status(), info.code()) {
        (Some(429), Some("rate_limit_error")) => spend_limit(parsed.as_ref()),
        (Some(400), Some("invalid_request_error")) => {
            message_of(parsed.as_ref()).and_then(|message| invalid_request(reasoning, message))
        }
        _ => None,
    };
    by_text.or_else(|| by_code(info))
}

fn message_of(parsed: Option<&Value>) -> Option<&str> {
    parsed?.get("error")?.get("message")?.as_str()
}

fn spend_limit(parsed: Option<&Value>) -> Option<ClientError> {
    let detail = parsed?
        .get("error")?
        .get("details")?
        .get("error_code")?
        .as_str()?;
    (detail == SPEND_LIMIT_DETAIL).then_some(ClientError::QuotaExceeded)
}

fn invalid_request(reasoning: ReasoningLevel, message: &str) -> Option<ClientError> {
    if USAGE_LIMIT_PREFIXES
        .iter()
        .any(|prefix| message.starts_with(prefix))
    {
        return Some(ClientError::QuotaExceeded);
    }
    if message.starts_with(CONTEXT_PREFIX) {
        return Some(ClientError::ContextTooLong { limit: None });
    }
    if REASONING_PREFIXES
        .iter()
        .any(|prefix| message.starts_with(prefix))
    {
        return Some(ClientError::ReasoningNotSupported {
            level: reasoning,
            reason: message.to_owned(),
        });
    }
    None
}

fn by_code(info: &ErrorInfo) -> Option<ClientError> {
    let limit_exceeded = || ClientError::LimitExceeded {
        kind: LimitKind::Other,
        description: info.text().to_owned(),
    };
    match (info.code(), info.status()) {
        (Some("overloaded_error"), _) => Some(ClientError::Unavailable { retry_after: None }),
        (Some("rate_limit_error"), _) => Some(ClientError::RateLimited { retry_after: None }),
        (Some("billing_error"), _) => Some(ClientError::QuotaExceeded),
        (Some("request_too_large"), _) => Some(limit_exceeded()),
        (Some("authentication_error"), _) => Some(ClientError::Authentication),
        (Some("permission_error"), _) => Some(ClientError::PermissionDenied),
        (None, Some(402)) => Some(ClientError::QuotaExceeded),
        (None, Some(413)) => Some(limit_exceeded()),
        _ => None,
    }
}
