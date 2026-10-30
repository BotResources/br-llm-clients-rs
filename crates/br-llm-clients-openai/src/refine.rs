use br_llm_clients_common_call::ErrorInfo;
use br_llm_clients_core::ClientError;

const MODEL_NOT_FOUND: &str = "model_not_found";
const CONTEXT_LENGTH_EXCEEDED: &str = "context_length_exceeded";

pub(crate) fn refine(info: &ErrorInfo) -> Option<ClientError> {
    match (info.status(), info.code()) {
        (Some(404), Some(MODEL_NOT_FOUND)) => Some(ClientError::ModelNotFound),
        (_, Some(CONTEXT_LENGTH_EXCEEDED)) => Some(ClientError::ContextTooLong { limit: None }),
        _ => None,
    }
}
