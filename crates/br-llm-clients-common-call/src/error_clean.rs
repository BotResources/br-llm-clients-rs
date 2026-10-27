use br_llm_clients_core::ClientError;
use br_llm_messages::ResponseEnding;

use crate::text_clean::Secrets;

pub(crate) fn clean_error(error: ClientError, secrets: &Secrets) -> ClientError {
    let clean = |text: String| secrets.clean(&text);
    let clean_code = |code: Option<String>| code.map(|code| secrets.clean(&code));
    match error {
        ClientError::ReplyUnusable { ending } => ClientError::ReplyUnusable {
            ending: clean_ending(ending, secrets),
        },
        ClientError::InvalidMessages { reason } => ClientError::InvalidMessages {
            reason: clean(reason),
        },
        ClientError::ReasoningNotSupported { level, reason } => {
            ClientError::ReasoningNotSupported {
                level,
                reason: clean(reason),
            }
        }
        ClientError::LimitExceeded { kind, description } => ClientError::LimitExceeded {
            kind,
            description: clean(description),
        },
        ClientError::ToolSchemaRejected { tool, reason } => ClientError::ToolSchemaRejected {
            tool,
            reason: clean(reason),
        },
        ClientError::OutputShapeRejected { reason } => ClientError::OutputShapeRejected {
            reason: clean(reason),
        },
        ClientError::MalformedResponse { detail } => ClientError::MalformedResponse {
            detail: clean(detail),
        },
        ClientError::ProviderRejected { status, code, body } => ClientError::ProviderRejected {
            status,
            code: clean_code(code),
            body: clean(body),
        },
        ClientError::ProviderFailed { status, code, body } => ClientError::ProviderFailed {
            status,
            code: clean_code(code),
            body: clean(body),
        },
        other => other,
    }
}

fn clean_ending(ending: ResponseEnding, secrets: &Secrets) -> ResponseEnding {
    match &ending {
        ResponseEnding::Unclassified { reason, .. } => {
            ResponseEnding::unclassified(secrets.clean(reason))
                .unwrap_or_else(|_| ResponseEnding::finished())
        }
        ResponseEnding::StopSequence {
            sequence: Some(sequence),
            ..
        } => ResponseEnding::matched_stop_sequence(secrets.clean(sequence))
            .unwrap_or_else(|_| ResponseEnding::stop_sequence()),
        _ => ending,
    }
}
