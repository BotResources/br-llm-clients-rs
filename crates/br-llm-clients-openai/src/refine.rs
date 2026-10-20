use br_llm_clients_common_call::ErrorInfo;
use br_llm_clients_core::ClientError;

pub(crate) fn refine(info: &ErrorInfo) -> Option<ClientError> {
    match (info.status(), info.code()) {
        (Some(404), Some("model_not_found")) => Some(ClientError::ModelNotFound),
        (_, Some("context_length_exceeded")) => Some(ClientError::ContextTooLong { limit: None }),
        _ => None,
    }
}
