use br_llm_clients_common_call::{Dialect, ErrorInfo};
use br_llm_clients_core::ClientError;

use crate::error_object::{self, ErrorObject};

const IN_FLIGHT_BUDGET: &str = "openrouter_in_flight_budget";
const PAYMENT_REQUIRED: u16 = 402;

pub(crate) fn refine(route: &dyn Dialect, info: &ErrorInfo) -> Option<ClientError> {
    let object = error_object::read(info.text());
    object
        .as_ref()
        .and_then(|object| openrouter_row(info, object))
        .or_else(|| route.refine_error(info))
        .or_else(|| from_status(info, object.as_ref()?))
}

fn openrouter_row(info: &ErrorInfo, object: &ErrorObject) -> Option<ClientError> {
    if let Some(source) = object.limit_source.as_deref() {
        return Some(if source == IN_FLIGHT_BUDGET {
            ClientError::RateLimited { retry_after: None }
        } else {
            ClientError::QuotaExceeded
        });
    }
    if object.moderation {
        return Some(ClientError::ContentBlocked);
    }
    let typed = match object.error_type.as_deref() {
        Some("payment_required") => Some(ClientError::QuotaExceeded),
        Some("context_length_exceeded") => Some(ClientError::ContextTooLong { limit: None }),
        Some("content_policy_violation") => Some(ClientError::ContentBlocked),
        Some("rate_limit_exceeded") => Some(ClientError::RateLimited { retry_after: None }),
        Some("provider_overloaded" | "provider_unavailable") => {
            Some(ClientError::Unavailable { retry_after: None })
        }
        Some("authentication") => Some(ClientError::Authentication),
        Some("permission_denied") => Some(ClientError::PermissionDenied),
        _ => None,
    };
    typed.or_else(|| {
        (info.status() == Some(PAYMENT_REQUIRED) || object.status == Some(PAYMENT_REQUIRED))
            .then_some(ClientError::QuotaExceeded)
    })
}

fn from_status(info: &ErrorInfo, object: &ErrorObject) -> Option<ClientError> {
    if info.status().is_some() {
        return None;
    }
    let status = object.status?;
    let code = object.error_type.clone().or_else(|| object.code.clone());
    let body = info.text().to_owned();
    Some(match status {
        401 => ClientError::Authentication,
        403 => ClientError::PermissionDenied,
        429 => ClientError::RateLimited {
            retry_after: info.retry_after(),
        },
        503 | 529 => ClientError::Unavailable {
            retry_after: info.retry_after(),
        },
        400..=499 => ClientError::ProviderRejected {
            status: Some(status),
            code,
            body,
        },
        _ => ClientError::ProviderFailed {
            status: Some(status),
            code,
            body,
        },
    })
}
