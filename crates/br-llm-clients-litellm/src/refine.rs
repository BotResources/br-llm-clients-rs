use br_llm_clients_common_call::ErrorInfo;
use br_llm_clients_core::ClientError;

use crate::error_object::ErrorObject;

pub(crate) fn from_status(info: &ErrorInfo, object: &ErrorObject) -> Option<ClientError> {
    if info.status().is_some() {
        return None;
    }
    let status = object.status?;
    let body = info.text().to_owned();
    let code = object.kind.clone();
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
