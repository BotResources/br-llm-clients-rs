use std::time::Duration;

use br_llm_clients_core::ClientError;

use crate::dialect::Dialect;
use crate::error_info::ErrorInfo;
use crate::text_clean::Secrets;

pub(crate) fn classify_refusal(dialect: &dyn Dialect, info: ErrorInfo) -> ClientError {
    resolve(dialect, &info)
}

pub(crate) fn classify_stream_error(
    dialect: &dyn Dialect,
    secrets: &Secrets,
    info: ErrorInfo,
) -> ClientError {
    resolve(dialect, &prepare_stream_error(secrets, info))
}

fn prepare_stream_error(secrets: &Secrets, info: ErrorInfo) -> ErrorInfo {
    let mut prepared = ErrorInfo::new(secrets.clean(info.text()));
    if let Some(status) = info.status().filter(|status| (400..=599).contains(status)) {
        prepared = prepared.with_status(status);
    }
    if let Some(code) = info.code() {
        prepared = prepared.with_code(secrets.clean(code));
    }
    if let Some(delay) = info.retry_after() {
        prepared = prepared.with_retry_after(delay);
    }
    prepared
}

fn resolve(dialect: &dyn Dialect, info: &ErrorInfo) -> ClientError {
    match dialect.refine_error(info) {
        Some(refined) => with_delay(refined, info.retry_after()),
        None => generic(info),
    }
}

fn with_delay(error: ClientError, delay: Option<Duration>) -> ClientError {
    match error {
        ClientError::RateLimited { retry_after: None } => {
            ClientError::RateLimited { retry_after: delay }
        }
        ClientError::Unavailable { retry_after: None } => {
            ClientError::Unavailable { retry_after: delay }
        }
        other => other,
    }
}

fn generic(info: &ErrorInfo) -> ClientError {
    let status = info.status();
    let code = info.code().map(str::to_owned);
    let body = info.text().to_owned();
    match status {
        Some(401) => ClientError::Authentication,
        Some(403) => ClientError::PermissionDenied,
        Some(429) => ClientError::RateLimited {
            retry_after: info.retry_after(),
        },
        Some(503 | 529) => ClientError::Unavailable {
            retry_after: info.retry_after(),
        },
        Some(400..=499) => ClientError::ProviderRejected { status, code, body },
        Some(_) | None => ClientError::ProviderFailed { status, code, body },
    }
}
