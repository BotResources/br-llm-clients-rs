use br_llm_clients_common_call::ErrorInfo;
use br_llm_clients_core::ClientError;

use crate::api::Route;

const CONTENT_BLOCKED_CODES: [&str; 2] = ["content_filter", "content_policy_violation"];
const DEPLOYMENT_NOT_FOUND: &str = "DeploymentNotFound";
const CONTEXT_LENGTH_EXCEEDED: &str = "context_length_exceeded";
const NO_CAPACITY: &str = "no_capacity";

pub(crate) fn refine(route: Route, info: &ErrorInfo) -> Option<ClientError> {
    match (info.status(), info.code()) {
        (_, Some(code)) if CONTENT_BLOCKED_CODES.contains(&code) => {
            Some(ClientError::ContentBlocked)
        }
        (Some(404), Some(DEPLOYMENT_NOT_FOUND)) => Some(ClientError::ModelNotFound),
        (_, Some(CONTEXT_LENGTH_EXCEEDED)) => Some(ClientError::ContextTooLong { limit: None }),
        (None, Some(NO_CAPACITY)) if route == Route::Responses => Some(ClientError::Unavailable {
            retry_after: info.retry_after(),
        }),
        _ => None,
    }
}
