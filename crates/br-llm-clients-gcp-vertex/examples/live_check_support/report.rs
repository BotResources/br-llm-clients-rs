use br_llm_clients_core::{CallOutcome, ClientError};
use br_llm_messages::{ContentBlock, ModelResponse};

pub fn print_request_id(outcome: &CallOutcome) {
    if let Some(id) = outcome.report().request_id() {
        println!("  request id: {id}");
    }
}

pub fn has_text(response: &ModelResponse) -> bool {
    response.blocks().iter().any(
        |block| matches!(block, ContentBlock::Text(text) if !text.text().as_str().trim().is_empty()),
    )
}

pub fn variant(error: &ClientError) -> &'static str {
    match error {
        ClientError::ModelNotFound => "ModelNotFound",
        ClientError::ModelNotAccessible => "ModelNotAccessible",
        ClientError::Authentication => "Authentication",
        ClientError::PermissionDenied => "PermissionDenied",
        ClientError::ContextTooLong { .. } => "ContextTooLong",
        ClientError::ContentBlocked => "ContentBlocked",
        ClientError::ReplyUnusable { .. } => "ReplyUnusable",
        ClientError::InvalidMessages { .. } => "InvalidMessages",
        ClientError::QuotaExceeded => "QuotaExceeded",
        ClientError::RateLimited { .. } => "RateLimited",
        ClientError::Unavailable { .. } => "Unavailable",
        ClientError::StreamTruncated => "StreamTruncated",
        ClientError::Transport => "Transport",
        ClientError::ReasoningNotSupported { .. } => "ReasoningNotSupported",
        ClientError::StructuredOutputNotSupported => "StructuredOutputNotSupported",
        ClientError::StructuredOutputWithToolsNotSupported => {
            "StructuredOutputWithToolsNotSupported"
        }
        ClientError::ToolsNotSupported => "ToolsNotSupported",
        ClientError::CacheNotSupported => "CacheNotSupported",
        ClientError::ImageNotSupported => "ImageNotSupported",
        ClientError::ImageMediaTypeNotSupported { .. } => "ImageMediaTypeNotSupported",
        ClientError::LimitExceeded { .. } => "LimitExceeded",
        ClientError::MaxOutputTokensTooLarge { .. } => "MaxOutputTokensTooLarge",
        ClientError::ToolSchemaRejected { .. } => "ToolSchemaRejected",
        ClientError::OutputShapeRejected { .. } => "OutputShapeRejected",
        ClientError::MalformedResponse { .. } => "MalformedResponse",
        ClientError::ProviderRejected { .. } => "ProviderRejected",
        ClientError::ProviderFailed { .. } => "ProviderFailed",
        _ => "a variant this example does not know",
    }
}
