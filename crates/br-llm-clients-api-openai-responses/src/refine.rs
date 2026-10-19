use br_llm_clients_common_call::ErrorInfo;
use br_llm_clients_core::ClientError;

const QUOTA_CODES: [&str; 5] = [
    "insufficient_quota",
    "credit_balance_exhausted",
    "organization_spend_limit_exceeded",
    "project_spend_limit_exceeded",
    "organization_usage_limit_exceeded",
];

const RATE_LIMIT_CODES: [&str; 2] = ["rate_limit_exceeded", "slow_down"];

const CONTENT_BLOCKED_CODES: [&str; 3] = [
    "bio_policy",
    "misalignment_policy_violation",
    "image_content_policy_violation",
];

const BLAMES_THE_REQUEST_CODES: [&str; 15] = [
    "invalid_prompt",
    "data_residency_mismatch",
    "invalid_image",
    "invalid_image_format",
    "invalid_base64_image",
    "invalid_image_url",
    "image_too_large",
    "image_too_small",
    "image_parse_error",
    "invalid_image_mode",
    "image_file_too_large",
    "unsupported_image_media_type",
    "empty_image_file",
    "failed_to_download_image",
    "image_file_not_found",
];

pub(crate) fn refine_error(info: &ErrorInfo) -> Option<ClientError> {
    let code = info.code()?;
    if QUOTA_CODES.contains(&code) {
        return Some(ClientError::QuotaExceeded);
    }
    if RATE_LIMIT_CODES.contains(&code) {
        return Some(ClientError::RateLimited {
            retry_after: info.retry_after(),
        });
    }
    if code == "server_is_overloaded" {
        return Some(ClientError::Unavailable {
            retry_after: info.retry_after(),
        });
    }
    if CONTENT_BLOCKED_CODES.contains(&code) {
        return Some(ClientError::ContentBlocked);
    }
    if info.status().is_none() && BLAMES_THE_REQUEST_CODES.contains(&code) {
        return Some(ClientError::ProviderRejected {
            status: None,
            code: Some(code.to_owned()),
            body: info.text().to_owned(),
        });
    }
    None
}
