use br_llm_clients_common_call::{Dialect, ErrorInfo};
use br_llm_clients_core::ClientError;

use crate::error_object::{self, ErrorObject};

const BUDGET_EXCEEDED: &str = "budget_exceeded";
const MODEL_ACCESS_DENIED_SUFFIX: &str = "_model_access_denied";

pub(crate) fn refine(route: &dyn Dialect, info: &ErrorInfo) -> Option<ClientError> {
    let object = error_object::read(info.text());
    object
        .as_ref()
        .and_then(proxy_row)
        .or_else(|| route.refine_error(info))
        .or_else(|| from_status(info, object.as_ref()?))
}

fn proxy_row(object: &ErrorObject) -> Option<ClientError> {
    let kind = object.kind.as_deref()?;
    if kind == BUDGET_EXCEEDED {
        Some(ClientError::QuotaExceeded)
    } else if kind.ends_with(MODEL_ACCESS_DENIED_SUFFIX) {
        Some(ClientError::ModelNotAccessible)
    } else {
        None
    }
}

fn from_status(info: &ErrorInfo, object: &ErrorObject) -> Option<ClientError> {
    if info.status().is_some() {
        return None;
    }
    let status = object.status?;
    let code = object.kind.clone();
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
